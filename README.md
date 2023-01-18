[<img alt="github" src="https://img.shields.io/badge/github-vivekpanyam/lunchbox-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/vivekpanyam/lunchbox)
[<img alt="crates.io" src="https://img.shields.io/crates/v/lunchbox.svg?style=for-the-badge&logo=rust" height="20">](https://crates.io/crates/lunchbox)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-lunchbox-66c2a5?style=for-the-badge&logo=docs.rs" height="20">](https://docs.rs/lunchbox)

An async virtual filesystem interface in Rust.

Lunchbox provides a common interface that can be used to interact with any filesystem (e.g. a local FS, in-memory FS, zip filesystem, etc). This interface closely matches `tokio::fs::` so it's easy to get started with.

High level features:
- Support for read only filesystems and read/write filesystems
- A built-in implementation for local filesystems
- An async interface


# Getting started

Add lunchbox to your Cargo.toml:

```toml
[dependencies]
lunchbox = "0.1"
```

## Step 1: Load a filesystem

At its core, Lunchbox provides a `ReadableFileSystem` and a `WritableFileSystem` trait. These traits provide analogues for all the functions available in `tokio::fs`.

We'll use the built in `LocalFS`, but you can use anything that implements one of the filesystem traits above.

```rust
use lunchbox::ReadableFileSystem;
use lunchbox::LocalFS;

// Create a lunchbox filesystem that uses the root of your local filesystem as its root
let local_fs = LocalFS::new();

// Create a lunchbox filesystem that uses `/tmp` as its root.
// `/some/path` inside the filesystem would be translated to `/tmp/some/path`
// on your local filesystem
let local_fs = LocalFS::with_base_dir("/tmp");
```

*Note: using `LocalFS` requires the `localfs` feature*

## Step 2: Use it instead of `tokio::fs::...`

Instead of
```rust
tokio::fs::canonicalize("/some/path").await
```

you'd write

```rust
local_fs.canonicalize("/some/path").await
```

# Details

## `Path`s

We can't use `std::path::Path` directly because it contains methods that directly access the filesystem (e.g. `exists`) and is not portable across OSs (e.g. a path on Windows is different than a path on Unix).

As an alternative, we use the [relative_path](https://docs.rs/relative-path/1.7.2/relative_path/index.html) crate to give us platform-independent paths. We also provide an extension trait (`LunchboxPathUtils`) to add methods like `exists`. This is available as `lunchbox::path::Path` and `lunchbox::path::PathBuf`.

Methods in the lunchbox traits generally accept anything that implements `AsRef<lunchbox::path::Path>`. This means you can use `str` or `String` directly as you would with standard library paths. See the [relative_path](https://docs.rs/relative-path/1.7.2/relative_path/index.html) docs for more details.

## Trait methods

In the below methods, note that `PathType` is just an alias for `AsRef<lunchbox::path::Path> + Send`.

`ReadableFileSystem` contains the following methods
```rust
// Open a file
async fn open(&self, path: impl PathType) -> Result<Self::FileType>
where
    Self::FileType: ReadableFile;

// These are almost identical to tokio::fs::...
async fn canonicalize(&self, path: impl PathType) -> Result<PathBuf>;
async fn metadata(&self, path: impl PathType) -> Result<Metadata>;
async fn read(&self, path: impl PathType) -> Result<Vec<u8>>;
async fn read_dir(&self, path: impl PathType) -> Result</* snip */>;
async fn read_link(&self, path: impl PathType) -> Result<PathBuf>;
async fn read_to_string(&self, path: impl PathType) -> Result<String>;
async fn symlink_metadata(&self, path: impl PathType) -> Result<Metadata>;
```

`WritableFileSystem` contains
```rust
// Create a file
async fn create(&self, path: impl PathType) -> Result<Self::FileType>
where
    Self::FileType: WritableFile;

// Open a file with options
async fn open_with_opts(
    &self,
    opts: &OpenOptions,
    path: impl PathType,
) -> Result<Self::FileType>
where
    Self::FileType: WritableFile;

// These are almost identical to tokio::fs::...
async fn copy(&self, from: impl PathType, to: impl PathType) -> Result<u64>;
async fn create_dir(&self, path: impl PathType) -> Result<()>;
async fn create_dir_all(&self, path: impl PathType) -> Result<()>;
async fn hard_link(&self, src: impl PathType, dst: impl PathType) -> Result<()>;
async fn remove_dir(&self, path: impl PathType) -> Result<()>;
async fn remove_dir_all(&self, path: impl PathType) -> Result<()>;
async fn remove_file(&self, path: impl PathType) -> Result<()>;
async fn rename(&self, from: impl PathType, to: impl PathType) -> Result<()>;
async fn set_permissions(&self, path: impl PathType, perm: Permissions) -> Result<()>;
async fn symlink(&self, src: impl PathType, dst: impl PathType) -> Result<()>;
async fn write(&self, path: impl PathType, contents: impl AsRef<[u8]> + Send) -> Result<()>;
```

These work with `ReadableFile`s and `WritableFile`s respectively:

```rust
/// A readable file
#[async_trait]
pub trait ReadableFile: AsyncRead
where
    Self: Sized,
{
    // These are almost identical to tokio::fs::File::...
    async fn metadata(&self) -> Result<Metadata>;
    async fn try_clone(&self) -> Result<Self>;
}

/// A file that supports both reads and writes
#[async_trait]
pub trait WritableFile: ReadableFile + AsyncWrite {
    // These are almost identical to tokio::fs::File::...
    async fn sync_all(&self) -> Result<()>;
    async fn sync_data(&self) -> Result<()>;
    async fn set_len(&self, size: u64) -> Result<()>;
    async fn set_permissions(&self, perm: Permissions) -> Result<()>;
}
```

Note that all `WritableFile`s must be `ReadableFile`s and all `WritableFileSystem`s must be `ReadableFileSystem`s as well.