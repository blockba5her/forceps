//! `forceps` is a crate that provides a simple and easy-to-use on-disk cache/database.
//!
//! **This crate is intended to be used with the [`tokio`](tokio) runtime.**
//!
//! `forceps` is made to be an easy-to-use, thread-safe, performant, and asynchronous disk cache
//! that has easy reading and manipulation of data. It levereges tokio's async `fs` APIs
//! and fast task schedulers to perform IO operations, and `sled` as a fast metadata database.
//!
//! It was originally designed to be used in [`scalpel`](https://github.com/blockba5her/scalpel),
//! the MD@Home implementation for the Rust language.
//!
//! ## Features
//!
//! - Asynchronous APIs
//! - Fast and reliable reading/writing
//! - Tuned for large-file databases
//! - Easily accessible value metadata
//! - Optimized for cache `HIT`s
//! - Easy error handling
//! - `bytes` crate support (non-optional)
//!
//! ## Database and Meta-database
//!
//! This database solution easily separates data into two databases: the LFS (large-file-storage)
//! database, and the metadata database. The LFS database is powered using Tokio's async filesystem
//! operations, whereas the metadata database is powered using [`sled`](sled).
//!
//! The advantage of splitting these two up is simple: Accessing metadata (for things like database
//! eviction) is realatively cheap and efficient, with the only downside being that `async` is not
//! present.
//!
//! # Examples
//!
//! ```rust
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! use forceps::Cache;
//!
//! let cache = Cache::new("./cache")
//!     .build()
//!     .await
//!     .unwrap();
//!
//! cache.write(b"MY_KEY", b"Hello World").await.unwrap();
//! let data = cache.read(b"MY_KEY").await.unwrap();
//! assert_eq!(data.as_ref(), b"Hello World");
//! # }
//! ```

#![warn(missing_docs)]

use std::io;

/// Global error type for the `forceps` crate, which is used in the `Result` types of all calls to
/// forcep APIs.
#[derive(Debug)]
pub enum ForcepError {
    /// An I/O operation error. This can occur on reads, writes, or builds.
    Io(io::Error),
    /// Error deserialization metadata information (most likely corrupted)
    MetaDe(bson::de::Error),
    /// Error serializing metadata information
    MetaSer(bson::ser::Error),
    /// Error with metadata sled database operation
    MetaDb(sled::Error),
    /// The entry for the specified key is not found
    NotFound,
}
/// Re-export of [`ForcepError`]
pub type Error = ForcepError;
/// Result that is returned by all error-bound operations of `forceps`.
pub type Result<T> = std::result::Result<T, ForcepError>;

impl std::fmt::Display for ForcepError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(fmt, "an I/O error occurred: {}", e),
            Self::MetaDe(e) => write!(fmt, "there was a problem deserializing metadata: {}", e),
            Self::MetaSer(e) => write!(fmt, "there was a problem serializing metadata: {}", e),
            Self::MetaDb(e) => write!(fmt, "an error with the metadata database occurred: {}", e),
            Self::NotFound => write!(fmt, "the entry for the key provided was not found"),
        }
    }
}
impl std::error::Error for ForcepError {}

mod tmp;

mod builder;
pub use builder::CacheBuilder;

mod cache;
pub use cache::Cache;

mod metadata;
pub(crate) use metadata::MetaDb;
pub use metadata::{Md5Bytes, Metadata};
