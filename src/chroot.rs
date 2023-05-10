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

use std::io::Result;
use std::sync::Arc;
use std::task::Poll;

use async_trait::async_trait;
use thiserror::Error;

use crate::types::{Permissions, WritableFile};
use crate::{
    path::{Path, PathBuf},
    types::{
        DirEntry, HasFileType, MaybeSend, MaybeSync, Metadata, OpenOptions, PathType, ReadDir,
        ReadDirPoller, ReadableFile,
    },
    ReadableFileSystem, WritableFileSystem,
};

/// Wraps a lunchbox filesystem to modify paths going into the filesystem
/// and paths coming out of the filesystem to add or remove a prefix respectively.
///
/// For example, `let p = fs.canonicalize("/some/path").await?;` would be approximately
/// translated to:
/// ```
/// let p = {
///     let path = base_dir.join("some/path");
///     let out = inner.canonicalize(path).await?;
///     out.strip_prefix(base_dir)
/// };
/// ```
///
/// Note: this should NOT be used for security purposes. It is intended for convenience.
pub struct ChrootFS<T> {
    inner: T,
    base_dir: PathBuf,
}

/// Used to support wrappers for a filesystem (e.g. Arc<T> where T is a filesystem)
pub trait FSWrapper {
    type F: HasFileType;

    fn get(&self) -> &Self::F;
}

impl<T> FSWrapper for &T
where
    T: HasFileType,
{
    type F = T;

    fn get(&self) -> &Self::F {
        self
    }
}

impl<T> FSWrapper for Arc<T>
where
    T: HasFileType,
{
    type F = T;

    fn get(&self) -> &Self::F {
        self.as_ref()
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ChrootError {
    /// Path `req` was not within the base directory `base`
    #[error("Path ({req}) was not within the base directory ({base})")]
    PathTranslationError { base: PathBuf, req: PathBuf },
}

impl<T> ChrootFS<T> {
    pub fn new(inner: T, base_dir: PathBuf) -> Self {
        Self { inner, base_dir }
    }

    /// Remove the base dir when returning paths
    fn strip_base_dir(&self, path: impl PathType) -> Result<PathBuf> {
        // Normalize the path. `clean` does this without touching the FS
        let path = path.as_ref();
        let cleaned = path_clean::clean(path.as_str());
        let cleaned = Path::new(&cleaned);

        // Attempt to strip the base_dir prefix
        cleaned
            .strip_prefix(&self.base_dir)
            .map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    ChrootError::PathTranslationError {
                        base: self.base_dir.clone(),
                        req: path.to_owned(),
                    },
                )
            })
            .map(|p| p.to_owned())
    }

    /// Add the base dir when taking paths as input
    fn with_base_dir(&self, path: impl PathType) -> Result<PathBuf> {
        // Join path to the base dir and normalize the path. `clean` does this
        // without touching the FS so we don't provide any additional information
        // by returning the cleaned path in error messages
        let cleaned = Path::new(&path_clean::clean(
            self.base_dir.join(path.as_ref()).as_str(),
        ))
        .to_owned();

        // Make sure `path` is within the base dir
        if cleaned.starts_with(&self.base_dir) {
            Ok(cleaned)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                ChrootError::PathTranslationError {
                    base: self.base_dir.clone(),
                    req: path.as_ref().to_owned(),
                },
            ))
        }
    }
}

impl<T: FSWrapper> HasFileType for ChrootFS<T> {
    type FileType = <<T as FSWrapper>::F as HasFileType>::FileType;
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl<T> ReadableFileSystem for ChrootFS<T>
where
    T: FSWrapper + MaybeSend + MaybeSync,
    T::F: ReadableFileSystem + MaybeSend + MaybeSync,
    <<T as FSWrapper>::F as HasFileType>::FileType: ReadableFile,
    <<T as FSWrapper>::F as ReadableFileSystem>::ReadDirPollerType: MaybeSend,
{
    async fn open(&self, path: impl PathType) -> Result<Self::FileType>
    where
        Self::FileType: ReadableFile,
    {
        let path = self.with_base_dir(path)?;
        self.inner.get().open(path).await
    }

    async fn canonicalize(&self, path: impl PathType) -> Result<PathBuf> {
        let path = self.with_base_dir(path)?;
        let out = self.inner.get().canonicalize(path).await?;

        self.strip_base_dir(out).map(|p| p.to_owned())
    }

    async fn metadata(&self, path: impl PathType) -> Result<Metadata> {
        let path = self.with_base_dir(path)?;
        self.inner.get().metadata(path).await
    }

    async fn read(&self, path: impl PathType) -> Result<Vec<u8>> {
        let path = self.with_base_dir(path)?;
        self.inner.get().read(path).await
    }

    type ReadDirPollerType =
        ChrootReadDirPoller<<<T as FSWrapper>::F as ReadableFileSystem>::ReadDirPollerType>;

    async fn read_dir(
        &self,
        path: impl PathType,
    ) -> Result<ReadDir<Self::ReadDirPollerType, Self>> {
        let path = self.with_base_dir(path)?;
        let inner_poller = self.inner.get().read_dir(path).await?.into_poller();
        let poller = ChrootReadDirPoller {
            inner: inner_poller,
        };

        Ok(ReadDir::new(poller, self))
    }

    async fn read_link(&self, path: impl PathType) -> Result<PathBuf> {
        let path = self.with_base_dir(path)?;
        let out = self.inner.get().read_link(path).await?;

        self.strip_base_dir(out).map(|p| p.to_owned())
    }

    async fn read_to_string(&self, path: impl PathType) -> Result<String> {
        let path = self.with_base_dir(path)?;
        self.inner.get().read_to_string(path).await
    }

    async fn symlink_metadata(&self, path: impl PathType) -> Result<Metadata> {
        let path = self.with_base_dir(path)?;
        self.inner.get().symlink_metadata(path).await
    }
}

pub struct ChrootReadDirPoller<P> {
    inner: P,
}

impl<T, P> ReadDirPoller<ChrootFS<T>> for ChrootReadDirPoller<P>
where
    T: FSWrapper,
    T::F: ReadableFileSystem,
    <<T as FSWrapper>::F as HasFileType>::FileType: ReadableFile,
    P: ReadDirPoller<T::F>,
    ChrootFS<T>: ReadableFileSystem,
    <ChrootFS<T> as HasFileType>::FileType: ReadableFile,
{
    fn poll_next_entry<'a>(
        &mut self,
        cx: &mut std::task::Context<'_>,
        fs: &'a ChrootFS<T>,
    ) -> std::task::Poll<Result<Option<crate::types::DirEntry<'a, ChrootFS<T>>>>> {
        match self.inner.poll_next_entry(cx, fs.inner.get()) {
            Poll::Ready(Ok(Some(entry))) => Poll::Ready(Ok(Some(DirEntry::new(
                fs,
                entry.file_name(),
                fs.strip_base_dir(entry.path()).map(|p| p.to_owned())?,
            )))),

            // We can pass through these cases
            Poll::Pending => Poll::Pending,
            Poll::Ready(Ok(None)) => Poll::Ready(Ok(None)),
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
        }
    }
}

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl<T> WritableFileSystem for ChrootFS<T>
where
    ChrootFS<T>: ReadableFileSystem,
    T: FSWrapper + MaybeSend + MaybeSync,
    T::F: WritableFileSystem + MaybeSend + MaybeSync,
    <<T as FSWrapper>::F as HasFileType>::FileType: WritableFile,

    // TODO: This shouldn't be necessary because these types
    // are equal.
    Self::FileType: From<<<T as FSWrapper>::F as HasFileType>::FileType>,
{
    async fn open_with_opts(
        &self,
        opts: &OpenOptions,
        path: impl PathType,
    ) -> Result<Self::FileType> {
        let path = self.with_base_dir(path)?;

        self.inner
            .get()
            .open_with_opts(opts, path)
            .await
            .map(|v| v.into())
    }

    async fn copy(&self, from: impl PathType, to: impl PathType) -> Result<u64> {
        let from = self.with_base_dir(from)?;
        let to = self.with_base_dir(to)?;

        self.inner.get().copy(from, to).await
    }

    async fn create_dir(&self, path: impl PathType) -> Result<()> {
        let path = self.with_base_dir(path)?;

        self.inner.get().create_dir(path).await
    }

    async fn create_dir_all(&self, path: impl PathType) -> Result<()> {
        let path = self.with_base_dir(path)?;

        self.inner.get().create_dir_all(path).await
    }

    async fn hard_link(&self, src: impl PathType, dst: impl PathType) -> Result<()> {
        let src = self.with_base_dir(src)?;
        let dst = self.with_base_dir(dst)?;

        self.inner.get().hard_link(src, dst).await
    }

    async fn remove_dir(&self, path: impl PathType) -> Result<()> {
        let path = self.with_base_dir(path)?;

        self.inner.get().remove_dir(path).await
    }

    async fn remove_dir_all(&self, path: impl PathType) -> Result<()> {
        let path = self.with_base_dir(path)?;

        self.inner.get().remove_dir_all(path).await
    }

    async fn remove_file(&self, path: impl PathType) -> Result<()> {
        let path = self.with_base_dir(path)?;

        self.inner.get().remove_file(path).await
    }

    async fn rename(&self, from: impl PathType, to: impl PathType) -> Result<()> {
        let from = self.with_base_dir(from)?;
        let to = self.with_base_dir(to)?;

        self.inner.get().rename(from, to).await
    }

    async fn set_permissions(&self, path: impl PathType, perm: Permissions) -> Result<()> {
        let path = self.with_base_dir(path)?;

        self.inner.get().set_permissions(path, perm).await
    }

    async fn symlink(&self, src: impl PathType, dst: impl PathType) -> Result<()> {
        let src = self.with_base_dir(src)?;
        let dst = self.with_base_dir(dst)?;

        self.inner.get().symlink(src, dst).await
    }

    async fn write(
        &self,
        path: impl PathType,
        contents: impl AsRef<[u8]> + MaybeSend,
    ) -> Result<()> {
        let path = self.with_base_dir(path)?;

        self.inner.get().write(path, contents).await
    }
}

#[cfg(test)]
mod tests {
    use super::ChrootFS;
    use crate::path::Path;
    use crate::LocalFS;
    use crate::ReadableFileSystem;
    use crate::WritableFileSystem;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    #[test]
    /// Ensures that joining "absolute" paths to a prefix does not overwrite the prefix
    fn test_path_join() {
        let a = Path::new("/a/b/c");
        let b = Path::new("/d/e/f");

        assert_eq!(a.join(b), Path::new("/a/b/c/d/e/f"))
    }

    #[cfg(feature = "localfs")]
    #[test]
    fn test_conversions() {
        let local = LocalFS::new().unwrap();
        let fs = ChrootFS::new(&local, "/".into());
        assert_eq!(fs.with_base_dir("/a/b/c").unwrap(), Path::new("/a/b/c"));
        assert_eq!(fs.with_base_dir("a/b/c").unwrap(), Path::new("/a/b/c"));

        let fs = ChrootFS::new(&local, "/tmp".into());
        assert_eq!(fs.with_base_dir("/a/b/c").unwrap(), Path::new("/tmp/a/b/c"));
        assert_eq!(fs.with_base_dir("a/b/c").unwrap(), Path::new("/tmp/a/b/c"));
        assert!(fs.with_base_dir("../etc/passwd").is_err());

        assert!(fs.strip_base_dir("/etc/passwd").is_err());
        assert!(fs.strip_base_dir("/a/b/c").is_err());
        assert!(fs.strip_base_dir("a/b/c").is_err());

        assert_eq!(
            fs.strip_base_dir("/tmp/a/b/c").unwrap(),
            Path::new("/a/b/c")
        );
        assert_eq!(fs.strip_base_dir("/tmp/a/b/c").unwrap(), Path::new("a/b/c"));
    }

    #[cfg(feature = "localfs")]
    #[tokio::test]
    async fn test_basic_chroot() {
        let fs = LocalFS::new().unwrap();

        // Create a test directory
        let tmpdir = tempfile::tempdir().unwrap();
        let tmpdir_path =
            crate::path::Path::new(tmpdir.path().to_str().unwrap().strip_prefix("/").unwrap());

        // Create a test file
        let mut file = fs.create(tmpdir_path.join("applesauce.txt")).await.unwrap();

        // Write some sample data to it
        file.write_all(b"some text").await.unwrap();
        file.flush().await.unwrap();

        // Open the file under chroot
        let chroot = ChrootFS::new(&fs, tmpdir_path.into());
        let mut file = chroot.open("applesauce.txt").await.unwrap();

        let mut buffer = String::new();
        file.read_to_string(&mut buffer).await.unwrap();

        assert_eq!(buffer, "some text");
    }
}
