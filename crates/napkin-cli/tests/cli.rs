//! The `napkin` binary end to end. Every run gets its own `$HOME`, so the user's extensions and
//! rate cache never leak in. The engine may still be a stub that only knows plain numbers, so the
//! expressions here are plain numbers and non-numeric words.

use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;

use tempfile::TempDir;

struct Sandbox {
    home: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            home: TempDir::new().expect("temp dir"),
        }
    }

    /// `napkin` with an isolated home and rate cache, reading English whatever the system
    /// language is.
    fn napkin(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_napkin"));
        command
            .env("HOME", self.home.path())
            .env("NAPKIN_LANGUAGE", "en")
            .env("XDG_DATA_HOME", self.home.path().join("data"))
            .env("XDG_CACHE_HOME", self.home.path().join("cache"))
            .arg("--rates-cache")
            .arg(self.home.path().join("rates.json"));
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.napkin()
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("napkin runs")
    }

    fn run_with_stdin(&self, args: &[&str], stdin: &str) -> Output {
        let mut child = self
            .napkin()
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("napkin starts");
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(stdin.as_bytes())
            .expect("write stdin");
        child.wait_with_output().expect("napkin finishes")
    }

    fn extension_dir(&self) -> PathBuf {
        let dir = self.home.path().join("extensions");
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("UTF-8 stderr")
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", stdout(output)))
}

#[test]
fn one_shot_prints_the_result() {
    let output = Sandbox::new().run(&["--no-extensions", "42"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "42\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn negative_numbers_are_expressions_not_flags() {
    let output = Sandbox::new().run(&["--no-extensions", "-5"]);
    assert_eq!(stdout(&output), "-5\n", "{}", stderr(&output));
}

#[test]
fn language_and_number_style_options() {
    let sandbox = Sandbox::new();
    let german = sandbox.run(&[
        "--no-extensions",
        "--language",
        "de",
        "--number-style",
        "german",
        "1 Monat in Tagen",
    ]);
    assert_eq!(stdout(&german), "30,42 Tage\n", "{}", stderr(&german));
    let auto = sandbox
        .napkin()
        .env("NAPKIN_LANGUAGE", "de_AT.UTF-8")
        .args(["--no-extensions", "1 Tag"])
        .output()
        .expect("napkin runs");
    assert_eq!(stdout(&auto), "1 Tag\n", "{}", stderr(&auto));
    let english = sandbox.run(&["--no-extensions", "--language", "en", "1 month in days"]);
    assert_eq!(stdout(&english), "30.42 days\n", "{}", stderr(&english));
    let unknown = sandbox.run(&["--language", "xx", "1"]);
    assert_eq!(unknown.status.code(), Some(2));
}

#[test]
fn one_shot_errors_exit_1_with_the_message_on_stderr() {
    let output = Sandbox::new().run(&["--no-extensions", "hello", "world"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).starts_with("napkin: "), "{}", stderr(&output));
}

#[test]
fn one_shot_json() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["--no-extensions", "--json", "2.5"]);
    assert!(output.status.success());
    assert_eq!(
        json(&output),
        serde_json::json!({ "input": "2.5", "result": { "text": "2.5", "number": 2.5, "unit": null }, "error": null })
    );

    let output = sandbox.run(&["--no-extensions", "--json", "hello"]);
    assert_eq!(output.status.code(), Some(1));
    let failed = json(&output);
    assert_eq!(failed["result"], serde_json::Value::Null);
    assert!(
        failed["error"].as_str().is_some_and(|error| !error.is_empty()),
        "{failed}"
    );
}

#[test]
fn document_from_stdin_prints_line_and_result() {
    let output = Sandbox::new().run_with_stdin(&["--no-extensions"], "1\n# Header\n2.5\r\n\nprose\n");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "1  → 1\n# Header\n2.5  → 2.5\n\nprose\n");
}

#[test]
fn results_only_prints_one_row_per_line() {
    let output = Sandbox::new().run_with_stdin(&["--no-extensions", "--results-only"], "1\nprose\n3");
    assert_eq!(stdout(&output), "1\n\n3\n");
}

#[test]
fn document_from_a_file_as_json() {
    let sandbox = Sandbox::new();
    let file = sandbox.home.path().join("budget.numi");
    fs::write(&file, "4\nrent\n").expect("write");
    let output = sandbox.run(&[
        "--no-extensions",
        "--json",
        "-f",
        file.to_str().expect("UTF-8 path"),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let lines = json(&output);
    assert_eq!(lines.as_array().map(Vec::len), Some(2));
    assert_eq!(lines[0]["result"]["text"], "4");
    assert_eq!(
        lines[1],
        serde_json::json!({ "input": "rent", "result": null, "error": null })
    );
}

#[test]
fn empty_documents_print_nothing() {
    let sandbox = Sandbox::new();
    assert_eq!(stdout(&sandbox.run_with_stdin(&["--no-extensions"], "")), "");
    assert_eq!(
        stdout(&sandbox.run_with_stdin(&["--no-extensions", "--json"], "")),
        "[]\n"
    );
}

#[test]
fn a_missing_file_is_an_error() {
    let output = Sandbox::new().run(&["--no-extensions", "-f", "/nonexistent/budget.numi"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("could not read"), "{}", stderr(&output));
}

#[test]
fn invalid_arguments_exit_2() {
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.run(&["--precision", "11", "1"]).status.code(), Some(2));
    assert_eq!(
        sandbox.run(&["--json", "--results-only", "1"]).status.code(),
        Some(2)
    );
    assert_eq!(sandbox.run(&["-f", "doc.numi", "1"]).status.code(), Some(2));
    assert!(
        sandbox
            .run(&["--precision", "0", "--no-extensions", "1"])
            .status
            .success()
    );
}

#[test]
fn broken_extensions_warn_but_never_stop_a_calculation() {
    let sandbox = Sandbox::new();
    let dir = sandbox.extension_dir();
    fs::write(dir.join("broken.js"), "throw new Error('nope');").expect("write");
    fs::write(
        dir.join("chatty.js"),
        "log('hello from chatty'); numi.setVariable('answer', 42);",
    )
    .expect("write");
    let dir = dir.to_str().expect("UTF-8 path");

    let output = sandbox.run(&["--extensions-dir", dir, "7"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output), "7\n");
    let warnings = stderr(&output);
    assert!(warnings.contains("broken.js failed: Error: nope"), "{warnings}");
    assert!(
        !warnings.contains("hello from chatty"),
        "logs only with --verbose: {warnings}"
    );

    let verbose = stderr(&sandbox.run(&["--extensions-dir", dir, "--verbose", "7"]));
    assert!(verbose.contains("chatty.js: hello from chatty"), "{verbose}");
    assert!(verbose.contains("loaded extension"), "{verbose}");

    assert_eq!(
        stderr(&sandbox.run(&["--extensions-dir", dir, "--no-extensions", "7"])),
        ""
    );
}

#[test]
fn extensions_load_from_napkins_folder_by_default() {
    let sandbox = Sandbox::new();
    let napkin_dir = if cfg!(target_os = "macos") {
        sandbox
            .home
            .path()
            .join("Library/Application Support/Napkin/extensions")
    } else {
        sandbox.home.path().join("data/Napkin/extensions")
    };
    fs::create_dir_all(&napkin_dir).expect("mkdir");
    fs::write(napkin_dir.join("broken.js"), "syntax error here").expect("write");
    let output = sandbox.run(&["1"]);
    assert!(output.status.success());
    assert!(
        stderr(&output).contains("broken.js failed: SyntaxError"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_closed_stdout_pipe_is_not_an_error() {
    let mut child = Sandbox::new()
        .napkin()
        .arg("--no-extensions")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("napkin starts");
    drop(child.stdout.take());
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let writer = thread::spawn(move || {
        let document = "1\n".repeat(200_000);
        // The child may exit before reading everything; a failed write is fine here.
        let _ = stdin.write_all(document.as_bytes());
    });
    let output = child.wait_with_output().expect("napkin finishes");
    writer.join().expect("writer thread");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
}

/// `napkin serve --port 0`, stopped when dropped.
struct Server {
    child: Child,
    address: String,
}

impl Server {
    fn start(sandbox: &Sandbox) -> Self {
        let mut child = sandbox
            .napkin()
            .args(["--no-extensions", "serve", "--port", "0"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("server starts");
        let mut banner = String::new();
        BufReader::new(child.stderr.take().expect("stderr is piped"))
            .read_line(&mut banner)
            .expect("banner");
        let address = banner
            .split("http://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or_else(|| panic!("no address in {banner:?}"))
            .to_owned();
        Self { child, address }
    }

    /// Sends a raw HTTP/1.0 request and returns the status code and body.
    fn request(&self, method: &str, target: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(&self.address).expect("connects");
        write!(stream, "{method} {target} HTTP/1.0\r\nHost: localhost\r\n\r\n").expect("request");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("response");
        let status = response
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .expect("status line");
        let body = response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.to_owned())
            .unwrap_or_default();
        (status, body)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn serve_answers_numis_alfred_requests() {
    let sandbox = Sandbox::new();
    let server = Server::start(&sandbox);
    assert!(
        server.address.starts_with("127.0.0.1:"),
        "binds to localhost only: {}",
        server.address
    );
    assert_eq!(server.request("GET", "/?q=%2042"), (200, "42".to_owned()));
    assert_eq!(
        server.request("GET", "?q=1.5").0,
        200,
        "Numi's workflow omits the slash"
    );
    let (status, body) = server.request("GET", "/?q=hello%20world");
    assert_eq!(status, 422);
    assert_ne!(body, "", "the error message is the body");
    assert_eq!(server.request("GET", "/").0, 400);
    assert_eq!(server.request("POST", "/?q=1").0, 405);
}

#[test]
fn serve_reports_a_busy_port() {
    let sandbox = Sandbox::new();
    let server = Server::start(&sandbox);
    let port = server.address.rsplit(':').next().expect("port");
    let output = sandbox.run(&["--no-extensions", "serve", "--port", port]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("could not listen"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_custom_rate_cache_is_used_and_left_alone() {
    let sandbox = Sandbox::new();
    let cache: &Path = &sandbox.home.path().join("rates.json");
    fs::write(cache, "not json").expect("write");
    let output = sandbox.run(&["--no-extensions", "3"]);
    assert!(
        output.status.success(),
        "an unusable cache falls back to the bundled rates"
    );
    assert_eq!(
        fs::read_to_string(cache).expect("read"),
        "not json",
        "no refresh without --refresh-rates"
    );
}
