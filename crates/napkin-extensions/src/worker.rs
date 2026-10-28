//! The thread that owns the `QuickJS` runtime.
//!
//! `QuickJS` values belong to their runtime and are neither `Send` nor `Sync`: rquickjs'
//! `Persistent` (the only way to keep a JavaScript function past one `Context::with` call) holds a
//! raw runtime pointer even with the `parallel` feature. Rather than asserting thread safety with
//! `unsafe impl Send`, one worker thread owns the runtime, every context and every saved function,
//! and the rest of the program talks to it over a channel. [`Worker`] is the cheap, `Send + Sync`
//! handle; the thread ends when the last handle (the loader or a registered custom function) is
//! dropped.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use napkin_core::{CustomUnit, ExtValue};
use rquickjs::context::EvalOptions;
use rquickjs::{CatchResultExt as _, Context, Ctx, Function, Persistent, Runtime, Value};

use crate::numi_api::{self, Declarations, SharedCollector};

/// How long a script's top-level code may run while loading.
const LOAD_TIMEOUT: Duration = Duration::from_secs(2);

/// How long one custom function call may run. The engine evaluates on every keystroke, so a
/// runaway extension must not freeze the editor for long.
const CALL_TIMEOUT: Duration = Duration::from_millis(500);

/// How long a caller waits for the worker to answer, including time queued behind other calls.
/// Only reached when the runtime is wedged in something the interrupt handler cannot stop.
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

/// Heap limit for all extensions together; a script that allocates more gets an exception.
const MEMORY_LIMIT: usize = 64 * 1024 * 1024;

/// `QuickJS` stack limit; deeper recursion throws a `RangeError` instead of crashing.
const JS_STACK_LIMIT: usize = 1024 * 1024;

/// Native stack of the worker thread, comfortably above [`JS_STACK_LIMIT`].
const THREAD_STACK: usize = 8 * 1024 * 1024;

const STOPPED: &str = "the extension runtime has stopped";

/// A handle to the JavaScript worker thread. Cloning is cheap; the thread lives while any clone does.
#[derive(Clone)]
pub(crate) struct Worker {
    jobs: Sender<Job>,
}

impl fmt::Debug for Worker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Worker").finish_non_exhaustive()
    }
}

/// Identifies a custom function inside the worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionHandle(usize);

/// A custom function declared by a script that loaded successfully.
#[derive(Debug)]
pub(crate) struct LoadedFunction {
    pub(crate) id: String,
    pub(crate) phrases: Vec<String>,
    pub(crate) handle: FunctionHandle,
}

/// What a script that loaded successfully declared.
#[derive(Debug, Default)]
pub(crate) struct Loaded {
    pub(crate) units: Vec<CustomUnit>,
    pub(crate) functions: Vec<LoadedFunction>,
    pub(crate) variables: Vec<(String, ExtValue)>,
}

/// The result of loading one script, plus its `log` output (kept even when the script failed).
#[derive(Debug)]
pub(crate) struct LoadOutcome {
    pub(crate) result: Result<Loaded, String>,
    pub(crate) log: Vec<String>,
}

enum Job {
    Load {
        name: String,
        source: String,
        reply: SyncSender<LoadOutcome>,
    },
    Call {
        handle: FunctionHandle,
        args: Vec<ExtValue>,
        reply: SyncSender<Result<ExtValue, String>>,
    },
}

impl Worker {
    /// Starts the thread and creates the runtime.
    pub(crate) fn spawn() -> Result<Self, String> {
        let (jobs, inbox) = mpsc::channel();
        let (ready, started) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("napkin-extensions".to_owned())
            .stack_size(THREAD_STACK)
            .spawn(move || run(&inbox, &ready))
            .map_err(|error| format!("could not start the extension thread: {error}"))?;
        started.recv().map_err(|_| STOPPED.to_owned())??;
        Ok(Self { jobs })
    }

    /// Runs `source` in a fresh context of its own and returns what it declared.
    pub(crate) fn load(&self, name: &str, source: String) -> LoadOutcome {
        let (reply, answer) = mpsc::sync_channel(1);
        let job = Job::Load {
            name: name.to_owned(),
            source,
            reply,
        };
        let result = self
            .jobs
            .send(job)
            .map_err(|_| STOPPED.to_owned())
            .and_then(|()| wait(&answer));
        result.unwrap_or_else(|error| LoadOutcome {
            result: Err(error),
            log: Vec::new(),
        })
    }

    /// Calls a custom function with `args`.
    pub(crate) fn call(&self, handle: FunctionHandle, args: &[ExtValue]) -> Result<ExtValue, String> {
        let (reply, answer) = mpsc::sync_channel(1);
        self.jobs
            .send(Job::Call {
                handle,
                args: args.to_vec(),
                reply,
            })
            .map_err(|_| STOPPED.to_owned())?;
        wait(&answer)?
    }
}

fn wait<T>(answer: &Receiver<T>) -> Result<T, String> {
    answer.recv_timeout(REPLY_TIMEOUT).map_err(|error| match error {
        RecvTimeoutError::Timeout => "the extension runtime did not answer in time".to_owned(),
        RecvTimeoutError::Disconnected => STOPPED.to_owned(),
    })
}

fn run(inbox: &Receiver<Job>, ready: &SyncSender<Result<(), String>>) {
    let mut host = match Host::new() {
        Ok(host) => host,
        Err(error) => {
            // The spawner is waiting on `ready`; if it is gone there is nobody to tell.
            let _ = ready.send(Err(error));
            return;
        }
    };
    if ready.send(Ok(())).is_err() {
        return;
    }
    // Ends when every `Worker` handle is gone.
    for job in inbox {
        // A caller that timed out has dropped its receiver; its late answer is discarded.
        match job {
            Job::Load { name, source, reply } => {
                let _ = reply.send(host.load(&name, source));
            }
            Job::Call { handle, args, reply } => {
                let _ = reply.send(host.call(handle, &args));
            }
        }
    }
}

/// A custom function kept alive in the context of the script that declared it.
struct HostedFunction {
    context: usize,
    function: Persistent<Function<'static>>,
}

/// The runtime and everything living in it. Only ever touched by the worker thread.
///
/// Field order is drop order: saved functions and contexts are released before the runtime,
/// which aborts the process if a persistent value outlives it.
struct Host {
    functions: Vec<HostedFunction>,
    contexts: Vec<Context>,
    deadline: Rc<Cell<Option<Instant>>>,
    runtime: Runtime,
}

impl Host {
    fn new() -> Result<Self, String> {
        let runtime =
            Runtime::new().map_err(|error| format!("could not create the JavaScript runtime: {error}"))?;
        runtime.set_memory_limit(MEMORY_LIMIT);
        runtime.set_max_stack_size(JS_STACK_LIMIT);
        let deadline: Rc<Cell<Option<Instant>>> = Rc::default();
        let interrupt = {
            let deadline = Rc::clone(&deadline);
            move || deadline.get().is_some_and(|deadline| Instant::now() >= deadline)
        };
        runtime.set_interrupt_handler(Some(Box::new(interrupt)));
        Ok(Self {
            functions: Vec::new(),
            contexts: Vec::new(),
            deadline,
            runtime,
        })
    }

    fn load(&mut self, name: &str, source: String) -> LoadOutcome {
        let context = match Context::full(&self.runtime) {
            Ok(context) => context,
            Err(error) => {
                return LoadOutcome {
                    result: Err(format!("could not create a context: {error}")),
                    log: Vec::new(),
                };
            }
        };
        let collector = SharedCollector::default();
        let evaluated = context.with(|ctx| {
            numi_api::install(&ctx, &collector)
                .catch(&ctx)
                .map_err(|error| numi_api::describe_error(&ctx, error))?;
            let mut options = EvalOptions::default();
            // Numi extensions are classic sloppy-mode scripts sharing one global scope per file.
            options.global = true;
            options.strict = false;
            options.filename = Some(name.to_owned());
            self.with_deadline(&ctx, LOAD_TIMEOUT, |ctx| {
                ctx.eval_with_options::<Value<'_>, _>(source, options)
            })
            .map(drop)
        });
        // Taken in every case, so no saved function stays behind in the collector.
        let Declarations {
            units,
            functions,
            variables,
            log,
        } = collector.borrow_mut().close();
        if let Err(error) = evaluated {
            return LoadOutcome {
                result: Err(error),
                log,
            };
        }
        let functions = if functions.is_empty() {
            Vec::new()
        } else {
            let context_index = self.contexts.len();
            self.contexts.push(context);
            functions
                .into_iter()
                .map(|declared| {
                    let handle = FunctionHandle(self.functions.len());
                    self.functions.push(HostedFunction {
                        context: context_index,
                        function: declared.function,
                    });
                    LoadedFunction {
                        id: declared.id,
                        phrases: declared.phrases,
                        handle,
                    }
                })
                .collect()
        };
        LoadOutcome {
            result: Ok(Loaded {
                units,
                functions,
                variables,
            }),
            log,
        }
    }

    fn call(&self, handle: FunctionHandle, args: &[ExtValue]) -> Result<ExtValue, String> {
        let hosted = self.functions.get(handle.0).ok_or("unknown extension function")?;
        let context = self
            .contexts
            .get(hosted.context)
            .ok_or("unknown extension context")?;
        context.with(|ctx| {
            let function = hosted
                .function
                .clone()
                .restore(&ctx)
                .map_err(|error| error.to_string())?;
            let arguments = numi_api::arguments_to_js(&ctx, args)
                .catch(&ctx)
                .map_err(|error| numi_api::describe_error(&ctx, error))?;
            let value = self.with_deadline(&ctx, CALL_TIMEOUT, |_| {
                function.call::<_, Value<'_>>((arguments,))
            })?;
            if value.is_promise() {
                return Err(
                    "the function returned a Promise; extension functions must be synchronous".to_owned(),
                );
            }
            numi_api::ext_value_from_js(&value).map_err(|message| format!("the function returned {message}"))
        })
    }

    /// Runs `body` with the interrupt deadline armed and turns an exception into a message.
    fn with_deadline<'js, T>(
        &self,
        ctx: &Ctx<'js>,
        limit: Duration,
        body: impl FnOnce(&Ctx<'js>) -> rquickjs::Result<T>,
    ) -> Result<T, String> {
        let deadline = Instant::now() + limit;
        self.deadline.set(Some(deadline));
        let result = body(ctx).catch(ctx);
        self.deadline.set(None);
        result.map_err(|error| {
            if Instant::now() >= deadline {
                format!("timed out after {} ms", limit.as_millis())
            } else {
                numi_api::describe_error(ctx, error)
            }
        })
    }
}
