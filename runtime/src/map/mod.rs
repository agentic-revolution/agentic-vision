//! SiteMap data structures, serialization, and query operations.
//!
//! The SiteMap is Cortex's primary data structure — a binary graph representing
//! an entire website with nodes (pages), edges (links), feature vectors (128 floats
//! per page), and action opcodes.

pub mod builder;
pub mod deserializer;
pub mod reader;
pub mod serializer;
pub mod types;

pub use builder::SiteMapBuilder;
pub use types::*;
