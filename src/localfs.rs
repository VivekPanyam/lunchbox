// Copyright 2023 Vivek Panyam
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.


//! Note this module requires the `localfs` feature (which is enabled by default)

use async_trait::async_trait;
use path_clean::PathClean;
use std::io::Result;
use std::path::{Path as StdPath, PathBuf as StdPathBuf};
use std::task::Poll;
use thiserror::Error;

use crate::types::{OpenOptions, OpenOptionsInner};
use crate::{
    path::{Path as LunchboxPath, PathBuf as LunchboxPathBuf},
    types::{
        DirEntry, HasFileType, Metadata, PathType, Permissions, ReadDir, ReadDirPoller,
        ReadableFile, WritableFile,
    },
    ReadableFileSystem, WritableFileSystem,
};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    /// The base directory `dir` doesn't exist or is not a directory
    #[error("Base directory doesn't exist or is not a directory: {dir}")]
    InvalidBaseDir { dir: StdPathBuf },

    /// Path `req` was not within the base directory `base`
    #[error("Path ({req}) was not within the base directory ({base})")]
    PathTranslationError { base: StdPathBuf, req: StdPathBuf },
}

/// A local filesystem
/// This implements both [`ReadableFileSystem`] and [`WritableFileSystem`]
///
/// ```
/// use lunchbox::ReadableFileSystem;
/// use lunchbox::LocalFS;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let fs = LocalFS::new()?;
///     let mut dir = fs.read_dir("/tmp/").await?;
///     while let Some(entry) = dir.next_entry().await? {
///         println!("{}", entry.file_name());
///     }
///
///     Ok(())
/// }
/// ```
#[derive(PartialEq, Debug)]
pub struct LocalFS {
    base_dir: StdPathBuf,
}

impl LocalFS {
    /// Create a new LocalFS using the root of the filesystem (i.e. "/") as a base directory
    ///
    /// ```
    /// use lunchbox::LocalFS;
    ///
    /// assert_eq!(LocalFS::new(), LocalFS::with_base_dir("/"));
    /// ```
    pub fn new() -> std::result::Result<LocalFS, Error> {
        Self::with_base_dir("/")
    }

    /// Create a new LocalFS given a base directory path. The returned filesystem will operate within the specified directory.
    /// Returns an error if the `base_dir` does not exist or is not a directory.
    ///
    /// ```
    /// use lunchbox::LocalFS;
    ///
    /// // Create a lunchbox filesystem that uses `/tmp` as its root.
    /// // `/some/path` inside the filesystem would be translated to `/tmp/some/path`
    /// // on the underlying filesystem
    /// let fs = LocalFS::with_base_dir("/tmp");
    /// ```
    ///
    /// Note: this should NOT be used for security purposes. It is intended for convenience.
    pub fn with_base_dir(base_dir: impl Into<StdPathBuf>) -> std::result::Result<LocalFS, Error> {
        let base_dir: StdPathBuf = base_dir.into();

        if !base_dir.is_dir() {
            return Err(Error::InvalidBaseDir { dir: base_dir });
        }

        Ok(LocalFS { base_dir })
    }

    fn to_std_path(&self, path: impl AsRef<LunchboxPath>) -> Result<StdPathBuf> {
        // Join path to the base dir and normalize the path. `clean` does this
        // without touching the FS so we don't provide any additional information
        // by returning the cleaned path in error messages
        let out = path.as_ref().to_path(&self.base_dir).clean();

        if out.starts_with(&self.base_dir) {
            Ok(out)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                Error::PathTranslationError {
                    base: self.base_dir.clone(),
                    req: out,
                },
            ))
        }
    }

    fn from_std_path(&self, path: impl AsRef<StdPath>) -> Result<LunchboxPathBuf> {
        // Make sure `path` is within the base dir and then strip the prefix
        let path = path.as_ref();
        if let Ok(relative) = path.strip_prefix(&self.base_dir) {
            LunchboxPathBuf::from_path(relative)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                Error::PathTranslationError {
                    base: self.base_dir.clone(),
                    req: path.to_path_buf(),
                },
            ))
        }
    }
}

impl HasFileType for LocalFS {
    /// The `FileType` for `LocalFS` is [`tokio::fs::File`]
    type FileType = tokio::fs::File;
}

#[async_trait]
impl ReadableFile for tokio::fs::File {
    async fn metadata(&self) -> Result<Metadata> {
        Ok(tokio::fs::File::metadata(&self).await?.into())
    }

    async fn try_clone(&self) -> Result<Self> {
        tokio::fs::File::try_clone(&self).await
    }
}

#[async_trait]
impl WritableFile for tokio::fs::File {
    async fn sync_all(&self) -> Result<()> {
        tokio::fs::File::sync_all(&self).await
    }

    async fn sync_data(&self) -> Result<()> {
        tokio::fs::File::sync_data(&self).await
    }

    async fn set_len(&self, size: u64) -> Result<()> {
        tokio::fs::File::set_len(&self, size).await
    }

    async fn set_permissions(&self, perm: Permissions) -> Result<()> {
        // Unfortunately, we can't directly create a new permissions instance
        let m = tokio::fs::File::metadata(&self).await?;
        let mut permissions = m.permissions();

        permissions.set_readonly(perm.readonly());

        tokio::fs::File::set_permissions(&self, permissions).await
    }
}

#[async_trait]
impl ReadableFileSystem for LocalFS {
    async fn open(&self, path: impl PathType) -> std::io::Result<Self::FileType> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::File::open(path).await
    }

    async fn canonicalize(&self, path: impl PathType) -> Result<LunchboxPathBuf> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        self.from_std_path(tokio::fs::canonicalize(path).await?)
    }

    async fn metadata(&self, path: impl PathType) -> Result<Metadata> {
        let f = self.open(path).await?;
        <tokio::fs::File as ReadableFile>::metadata(&f).await
    }

    async fn read(&self, path: impl PathType) -> Result<Vec<u8>> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::read(path).await
    }

    type ReadDirPollerType = LocalFSReadDirPoller;
    async fn read_dir(
        &self,
        path: impl PathType,
    ) -> Result<ReadDir<Self::ReadDirPollerType, Self>> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        let native = tokio::fs::read_dir(path).await?;
        let poller = LocalFSReadDirPoller { inner: native };
        Ok(ReadDir::new(poller, self))
    }

    async fn read_link(&self, path: impl PathType) -> Result<LunchboxPathBuf> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        self.from_std_path(tokio::fs::read_link(path).await?)
    }
    async fn read_to_string(&self, path: impl PathType) -> Result<String> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::read_to_string(path).await
    }

    async fn symlink_metadata(&self, path: impl PathType) -> Result<Metadata> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        Ok(tokio::fs::symlink_metadata(path).await?.into())
    }
}

#[doc(hidden)]
pub struct LocalFSReadDirPoller {
    inner: tokio::fs::ReadDir,
}

impl ReadDirPoller<LocalFS> for LocalFSReadDirPoller {
    fn poll_next_entry<'a>(
        &mut self,
        cx: &mut std::task::Context<'_>,
        fs: &'a LocalFS,
    ) -> Poll<std::io::Result<Option<DirEntry<'a, LocalFS>>>> {
        match self.inner.poll_next_entry(cx) {
            Poll::Ready(Ok(Some(entry))) => {
                let de = DirEntry::new(
                    fs,
                    entry.file_name().into_string().unwrap(),
                    fs.from_std_path(entry.path())?,
                );

                Poll::Ready(Ok(Some(de)))
            }

            // Pass through
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(None)) => Poll::Ready(Ok(None)),
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
        }
    }
}

#[async_trait]
impl WritableFileSystem for LocalFS {
    async fn open_with_opts(
        &self,
        opts: &OpenOptions,
        path: impl PathType,
    ) -> Result<Self::FileType> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        // Get the options
        let inner: OpenOptionsInner = opts.into();

        // Open with those options
        tokio::fs::OpenOptions::new()
            .append(inner.append)
            .create(inner.create)
            .create_new(inner.create_new)
            .read(inner.read)
            .truncate(inner.truncate)
            .write(inner.write)
            .open(path)
            .await
    }

    async fn copy(&self, from: impl PathType, to: impl PathType) -> Result<u64> {
        // Convert to std path
        let from = self.to_std_path(from)?;
        let to = self.to_std_path(to)?;

        tokio::fs::copy(from, to).await
    }

    async fn create_dir(&self, path: impl PathType) -> Result<()> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::create_dir(path).await
    }

    async fn create_dir_all(&self, path: impl PathType) -> Result<()> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::create_dir_all(path).await
    }

    async fn hard_link(&self, src: impl PathType, dst: impl PathType) -> Result<()> {
        // Convert to std path
        let src = self.to_std_path(src)?;
        let dst = self.to_std_path(dst)?;

        tokio::fs::hard_link(src, dst).await
    }

    async fn remove_dir(&self, path: impl PathType) -> Result<()> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::remove_dir(path).await
    }

    async fn remove_dir_all(&self, path: impl PathType) -> Result<()> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::remove_dir_all(path).await
    }

    async fn remove_file(&self, path: impl PathType) -> Result<()> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::remove_file(path).await
    }

    async fn rename(&self, from: impl PathType, to: impl PathType) -> Result<()> {
        // Convert to std path
        let from = self.to_std_path(from)?;
        let to = self.to_std_path(to)?;

        tokio::fs::rename(from, to).await
    }

    async fn set_permissions(&self, path: impl PathType, perm: Permissions) -> Result<()> {
        let f = self.open(path).await?;
        <tokio::fs::File as WritableFile>::set_permissions(&f, perm).await
    }

    async fn symlink(&self, src: impl PathType, dst: impl PathType) -> Result<()> {
        // Convert to std path
        let src = self.to_std_path(src)?;
        let dst = self.to_std_path(dst)?;

        tokio::fs::symlink(src, dst).await
    }

    async fn write(&self, path: impl PathType, contents: impl AsRef<[u8]> + Send) -> Result<()> {
        // Convert to std path
        let path = self.to_std_path(path)?;

        tokio::fs::write(path, contents).await
    }
}

#[cfg(test)]
mod tests {
    use crate::path::Path as LunchboxPath;
    use crate::LocalFS;
    use std::path::Path as StdPath;

    #[test]
    fn test_base_dir() {
        assert!(LocalFS::with_base_dir("").is_err());
        assert!(LocalFS::with_base_dir("/mostlikelynotarealdirectory").is_err());
        assert!(LocalFS::with_base_dir("/").is_ok());
    }

    #[test]
    fn test_new() {
        assert_eq!(LocalFS::new(), LocalFS::with_base_dir("/"));
    }

    #[test]
    fn test_conversions() {
        let fs = LocalFS::with_base_dir("/").unwrap();
        assert_eq!(fs.to_std_path("/a/b/c").unwrap(), StdPath::new("/a/b/c"));
        assert_eq!(fs.to_std_path("a/b/c").unwrap(), StdPath::new("/a/b/c"));

        let fs = LocalFS::with_base_dir("/tmp").unwrap();
        assert_eq!(
            fs.to_std_path("/a/b/c").unwrap(),
            StdPath::new("/tmp/a/b/c")
        );
        assert_eq!(fs.to_std_path("a/b/c").unwrap(), StdPath::new("/tmp/a/b/c"));
        assert!(fs.to_std_path("../etc/passwd").is_err());

        assert!(fs.from_std_path("/etc/passwd").is_err());
        assert!(fs.from_std_path("/a/b/c").is_err());
        assert!(fs.from_std_path("a/b/c").is_err());

        assert_eq!(
            fs.from_std_path("/tmp/a/b/c").unwrap(),
            LunchboxPath::new("/a/b/c")
        );
        assert_eq!(
            fs.from_std_path("/tmp/a/b/c").unwrap(),
            LunchboxPath::new("a/b/c")
        );
    }
}
