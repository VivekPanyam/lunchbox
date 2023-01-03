#![doc = include_str!(concat!(env!("OUT_DIR"), "/README_processed.md"))]

#[cfg(feature = "localfs")]
pub mod localfs;
pub mod path;
pub mod types;

pub use localfs::LocalFS;
pub use types::ReadableFileSystem;
pub use types::WritableFileSystem;
