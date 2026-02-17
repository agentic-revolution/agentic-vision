//! Pool manager for browser rendering contexts.
//!
//! Manages a pool of browser contexts, controlling concurrency
//! and reusing contexts when possible.

use crate::renderer::{RenderContext, Renderer};
use anyhow::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Handle to a borrowed browser context from the pool.
pub struct ContextHandle {
    context: Option<Box<dyn RenderContext>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
    active_count: Arc<AtomicUsize>,
}

impl ContextHandle {
    /// Get a reference to the render context.
    pub fn context(&self) -> &dyn RenderContext {
        self.context.as_ref().expect("context already taken").as_ref()
    }

    /// Get a mutable reference to the render context.
    pub fn context_mut(&mut self) -> &mut dyn RenderContext {
        self.context
            .as_mut()
            .expect("context already taken")
            .as_mut()
    }

    /// Take the context out of the handle (for passing to close).
    pub fn take(mut self) -> Box<dyn RenderContext> {
        self.context.take().expect("context already taken")
    }
}

impl Drop for ContextHandle {
    fn drop(&mut self) {
        self.active_count.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Manages a pool of browser contexts with concurrency limits.
pub struct PoolManager {
    renderer: Arc<dyn Renderer>,
    semaphore: Arc<Semaphore>,
    max_contexts: usize,
    active_count: Arc<AtomicUsize>,
}

impl PoolManager {
    /// Create a new pool manager.
    pub fn new(renderer: Arc<dyn Renderer>, max_contexts: usize) -> Self {
        Self {
            renderer,
            semaphore: Arc::new(Semaphore::new(max_contexts)),
            max_contexts,
            active_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Acquire a browser context from the pool.
    ///
    /// Blocks if the maximum number of concurrent contexts is reached.
    pub async fn acquire(&self) -> Result<ContextHandle> {
        let permit = Arc::clone(&self.semaphore)
            .acquire_owned()
            .await
            .map_err(|e| anyhow::anyhow!("semaphore closed: {}", e))?;

        let context = self.renderer.new_context().await?;
        self.active_count.fetch_add(1, Ordering::SeqCst);

        Ok(ContextHandle {
            context: Some(context),
            _permit: permit,
            active_count: Arc::clone(&self.active_count),
        })
    }

    /// Return a context to the pool (closes it).
    pub async fn release(&self, handle: ContextHandle) -> Result<()> {
        let context = handle.take();
        context.close().await
    }

    /// Number of currently active contexts.
    pub fn active(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Maximum allowed concurrent contexts.
    pub fn max_contexts(&self) -> usize {
        self.max_contexts
    }

    /// Available permits (slots for new contexts).
    pub fn available(&self) -> usize {
        self.semaphore.available_permits()
    }
}
