//! Human-like behavior simulation.
//!
//! Adds random delays between actions to appear more natural.

use rand::Rng;
use std::time::Duration;

/// Generate a random delay between min_ms and max_ms.
pub fn random_delay(min_ms: u64, max_ms: u64) -> Duration {
    let mut rng = rand::thread_rng();
    let ms = rng.gen_range(min_ms..=max_ms);
    Duration::from_millis(ms)
}

/// Standard inter-action delay (50-200ms).
pub fn action_delay() -> Duration {
    random_delay(50, 200)
}

/// Typing delay between characters (30-120ms).
pub fn typing_delay() -> Duration {
    random_delay(30, 120)
}

/// Page load wait (500-2000ms after navigation).
pub fn page_load_delay() -> Duration {
    random_delay(500, 2000)
}

/// Sleep for a random action delay.
pub async fn sleep_action_delay() {
    tokio::time::sleep(action_delay()).await;
}

/// Sleep for a random typing delay.
pub async fn sleep_typing_delay() {
    tokio::time::sleep(typing_delay()).await;
}
