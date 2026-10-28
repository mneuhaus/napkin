//! Blocking HTTPS downloads of the two rate sources.

use std::thread;
use std::time::Duration;

use ureq::Agent;

/// Upper bound for one download, connect to last byte. Both sources answer in well under a second;
/// a longer wait means the network is gone and the caller should fall back to the cache.
const TIMEOUT: Duration = Duration::from_secs(10);

const USER_AGENT: &str = concat!(
    "napkin/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/mneuhaus/napkin)"
);

/// Fetches `first` and `second` in parallel, each with its own result.
pub(crate) fn fetch_both(first: &str, second: &str) -> (Result<String, String>, Result<String, String>) {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .user_agent(USER_AGENT)
        .build()
        .into();
    thread::scope(|scope| {
        let first = scope.spawn(|| fetch(&agent, first));
        let second = fetch(&agent, second);
        let first = first
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        (first, second)
    })
}

fn fetch(agent: &Agent, url: &str) -> Result<String, String> {
    let mut response = agent.get(url).call().map_err(|error| error.to_string())?;
    response
        .body_mut()
        .read_to_string()
        .map_err(|error| error.to_string())
}
