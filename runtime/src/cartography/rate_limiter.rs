//! Rate limiter for polite crawling.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::Instant;

/// Rate limiter that enforces concurrency limits and minimum delays.
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    min_delay: Duration,
    last_request: tokio::sync::Mutex<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// - `max_concurrent`: maximum number of concurrent requests
    /// - `min_delay_ms`: minimum milliseconds between requests
    pub fn new(max_concurrent: usize, min_delay_ms: u64) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            min_delay: Duration::from_millis(min_delay_ms),
            last_request: tokio::sync::Mutex::new(Instant::now()),
        }
    }

    /// Create a rate limiter from robots.txt crawl delay.
    pub fn from_crawl_delay(crawl_delay: Option<f32>, max_concurrent: usize) -> Self {
        let delay_ms = crawl_delay.map(|d| (d * 1000.0) as u64).unwrap_or(100);
        Self::new(max_concurrent, delay_ms)
    }

    /// Acquire permission to make a request. Blocks until rate limit allows.
    pub async fn acquire(&self) -> RateLimitGuard {
        // Acquire semaphore permit
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();

        // Enforce minimum delay
        {
            let mut last = self.last_request.lock().await;
            let elapsed = last.elapsed();
            if elapsed < self.min_delay {
                tokio::time::sleep(self.min_delay - elapsed).await;
            }
            *last = Instant::now();
        }

        RateLimitGuard { _permit: permit }
    }
}

/// Guard that releases the rate limiter permit when dropped.
pub struct RateLimitGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(2, 10);
        let _g1 = limiter.acquire().await;
        let _g2 = limiter.acquire().await;
        // Both acquired successfully (max_concurrent=2)
    }

    #[tokio::test]
    async fn test_rate_limiter_from_crawl_delay() {
        let limiter = RateLimiter::from_crawl_delay(Some(0.5), 3);
        let _g = limiter.acquire().await;
        // Delay of 500ms is enforced between requests
    }
}
