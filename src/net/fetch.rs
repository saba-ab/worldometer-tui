//! `fetch(url) -> html`. Honest User-Agent, timeout, and a global rate limit of
//! at most one request per second (constraint C3).

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("network: {0}")]
    Http(#[from] reqwest::Error),
    #[error("HTTP {0}")]
    Status(u16),
}

pub const USER_AGENT: &str = concat!("worldo-tui/", env!("CARGO_PKG_VERSION"), " (personal use)");
const TIMEOUT: Duration = Duration::from_secs(20);
pub const MIN_INTERVAL: Duration = Duration::from_secs(1);

/// Time of the last request, shared by every fetcher in the process.
static LAST_REQUEST: Mutex<Option<Instant>> = Mutex::new(None);

pub struct Fetcher {
    client: reqwest::blocking::Client,
}

impl Fetcher {
    pub fn new() -> Result<Self, FetchError> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(TIMEOUT)
            .build()?;
        Ok(Self { client })
    }

    /// GETs `url` and returns the body text.
    pub fn fetch(&self, url: &str) -> Result<String, FetchError> {
        rate_limit();
        let response = self.client.get(url).send()?;
        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::Status(status.as_u16()));
        }
        Ok(response.text()?)
    }
}

/// How long to wait before the next request may start.
pub fn wait_needed(last: Option<Instant>, now: Instant, min_interval: Duration) -> Duration {
    match last {
        Some(last) => min_interval.saturating_sub(now.saturating_duration_since(last)),
        None => Duration::ZERO,
    }
}

/// Blocks until at least [`MIN_INTERVAL`] has passed since the previous request.
/// The lock is held while sleeping so concurrent callers queue up.
fn rate_limit() {
    let mut last = LAST_REQUEST
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let wait = wait_needed(*last, Instant::now(), MIN_INTERVAL);
    if !wait.is_zero() {
        thread::sleep(wait);
    }
    *last = Some(Instant::now());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_waits_out_the_remainder() {
        let now = Instant::now();
        let second = Duration::from_secs(1);
        assert_eq!(wait_needed(None, now, second), Duration::ZERO);
        let last = now - Duration::from_millis(300);
        assert_eq!(
            wait_needed(Some(last), now, second),
            Duration::from_millis(700)
        );
        let long_ago = now - Duration::from_secs(5);
        assert_eq!(wait_needed(Some(long_ago), now, second), Duration::ZERO);
    }

    #[test]
    fn user_agent_is_honest() {
        assert!(USER_AGENT.starts_with("worldo-tui/"));
        assert!(USER_AGENT.contains("personal use"));
    }
}
