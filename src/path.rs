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

use std::pin::Pin;

use async_trait::async_trait;

use crate::types::Metadata;
use crate::types::PathType;
use crate::types::ReadDir;
use crate::ReadableFileSystem;

pub type Path = relative_path::RelativePath;
pub type PathBuf = relative_path::RelativePathBuf;

// Sealed trait
mod private {
    pub trait Sealed {}
    impl Sealed for super::Path {}
}

pub type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// An extension trait that provides some methods defined on `std::path::Path` for `lunchbox::path::Path`
#[async_trait]
pub trait LunchboxPathUtils: private::Sealed
where
    Self: PathType + Sync,
{
    /// Equivalent to
    /// ```
    /// async fn metadata(&self, fs: &impl ReadableFileSystem) -> std::io::Result<Metadata>;
    /// ```
    ///
    /// An alias for [`fs.metadata(self)`](ReadableFileSystem::metadata)
    fn metadata<'a, 'b, 'c>(
        &'a self,
        fs: &'b impl ReadableFileSystem,
    ) -> BoxFuture<'c, std::io::Result<Metadata>>
    where
        'a: 'c,
        'b: 'c,
    {
        // Because `ReadableFileSystem` uses async_trait as well, we can save a Box::pin by
        // manually passing through to fs
        fs.metadata(self)
    }

    /// Equivalent to
    /// ```
    /// async fn symlink_metadata(&self, fs: &impl ReadableFileSystem) -> std::io::Result<Metadata>;
    /// ```
    ///
    /// An alias for [`fs.symlink_metadata(self)`](ReadableFileSystem::symlink_metadata)
    fn symlink_metadata<'a, 'b, 'c>(
        &'a self,
        fs: &'b impl ReadableFileSystem,
    ) -> BoxFuture<'c, std::io::Result<Metadata>>
    where
        'a: 'c,
        'b: 'c,
    {
        // Because `ReadableFileSystem` uses async_trait as well, we can save a Box::pin by
        // manually passing through to fs
        fs.symlink_metadata(self)
    }

    /// Equivalent to
    /// ```
    /// async fn canonicalize(&self, fs: &impl ReadableFileSystem) -> std::io::Result<PathBuf>;
    /// ```
    ///
    /// An alias for [`fs.canonicalize(self)`](ReadableFileSystem::canonicalize)
    fn canonicalize<'a, 'b, 'c>(
        &'a self,
        fs: &'b impl ReadableFileSystem,
    ) -> BoxFuture<'c, std::io::Result<PathBuf>>
    where
        'a: 'c,
        'b: 'c,
    {
        // Because `ReadableFileSystem` uses async_trait as well, we can save a Box::pin by
        // manually passing through to fs
        fs.canonicalize(self)
    }

    /// Equivalent to
    /// ```
    /// async fn read_link(&self, fs: &impl ReadableFileSystem) -> std::io::Result<PathBuf>;
    /// ```
    ///
    /// An alias for [`fs.read_link(self)`](ReadableFileSystem::read_link)
    fn read_link<'a, 'b, 'c>(
        &'a self,
        fs: &'b impl ReadableFileSystem,
    ) -> BoxFuture<'c, std::io::Result<PathBuf>>
    where
        'a: 'c,
        'b: 'c,
    {
        // Because `ReadableFileSystem` uses async_trait as well, we can save a Box::pin by
        // manually passing through to fs
        fs.read_link(self)
    }

    /// Equivalent to
    /// ```
    /// async fn read_dir<T: ReadableFileSystem>(
    ///    &self,
    ///    fs: &T,
    /// ) -> std::io::Result<ReadDir<T::ReadDirPollerType, T>>;
    /// ```
    ///
    /// An alias for [`fs.read_dir(self)`](ReadableFileSystem::read_dir)
    fn read_dir<'a, 'b, 'c, T: ReadableFileSystem>(
        &'a self,
        fs: &'b T,
    ) -> BoxFuture<'c, std::io::Result<ReadDir<'b, T::ReadDirPollerType, T>>>
    where
        'a: 'c,
        'b: 'c,
    {
        // Because `ReadableFileSystem` uses async_trait as well, we can save a Box::pin by
        // manually passing through to fs
        fs.read_dir(self)
    }

    /// Equivalent to
    /// ```
    /// async fn exists(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool;
    /// ```
    ///
    /// Returns `true` if the path points at an existing entity.
    ///
    /// Warning: this method has a risk of introducing time-of-check to time-of-use (TOCTOU) bugs.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file.
    ///
    /// If you cannot access the metadata of the file, e.g. because of a
    /// permission error or broken symbolic links, this will return `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::path::Path;
    /// assert!(!Path::new("does_not_exist.txt").exists(&fs));
    /// ```
    #[must_use]
    async fn exists(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool {
        fs.metadata(self).await.is_ok()
    }

    /// Equivalent to
    /// ```
    /// async fn is_file(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool;
    /// ```
    ///
    /// Returns `true` if the path exists on disk and is pointing at a regular file.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file.
    ///
    /// If you cannot access the metadata of the file, e.g. because of a
    /// permission error or broken symbolic links, this will return `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::path::Path;
    /// assert_eq!(Path::new("./is_a_directory/").is_file(&fs), false);
    /// assert_eq!(Path::new("a_file.txt").is_file(&fs), true);
    /// ```
    ///
    /// # See Also
    ///
    /// This is a convenience function that coerces errors to false. If you want to
    /// check errors, call [`ReadableFileSystem::metadata`] and handle its [`Result`]. Then call
    /// [`Metadata::is_file`] if it was [`Ok`].
    #[must_use]
    async fn is_file(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool {
        fs.metadata(self)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false)
    }

    /// Equivalent to
    /// ```
    /// async fn is_dir(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool;
    /// ```
    ///
    /// Returns `true` if the path exists on disk and is pointing at a directory.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file.
    ///
    /// If you cannot access the metadata of the file, e.g. because of a
    /// permission error or broken symbolic links, this will return `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::path::Path;
    /// assert_eq!(Path::new("./is_a_directory/").is_dir(&fs), true);
    /// assert_eq!(Path::new("a_file.txt").is_dir(&fs), false);
    /// ```
    ///
    /// # See Also
    ///
    /// This is a convenience function that coerces errors to false. If you want to
    /// check errors, call [`ReadableFileSystem::metadata`] and handle its [`Result`]. Then call
    /// [`Metadata::is_dir`] if it was [`Ok`].
    #[must_use]
    async fn is_dir(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool {
        fs.metadata(self).await.map(|m| m.is_dir()).unwrap_or(false)
    }

    /// Equivalent to
    /// ```
    /// async fn is_symlink(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool;
    /// ```
    ///
    /// Returns `true` if the path exists on disk and is pointing at a symbolic link.
    ///
    /// This function will not traverse symbolic links.
    /// In case of a broken symbolic link this will also return true.
    ///
    /// If you cannot access the directory containing the file, e.g., because of a
    /// permission error, this will return false.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::path::Path;
    /// use std::os::unix::fs::symlink;
    ///
    /// let link_path = Path::new("link");
    /// symlink("/origin_does_not_exist/", link_path).unwrap();
    /// assert_eq!(link_path.is_symlink(&fs), true);
    /// assert_eq!(link_path.exists(&fs), false);
    /// ```
    ///
    /// # See Also
    ///
    /// This is a convenience function that coerces errors to false. If you want to
    /// check errors, call [`ReadableFileSystem::symlink_metadata`] and handle its [`Result`]. Then call
    /// [`Metadata::is_symlink`] if it was [`Ok`].
    #[must_use]
    async fn is_symlink(&self, fs: &(impl ReadableFileSystem + Sync)) -> bool {
        fs.symlink_metadata(self)
            .await
            .map(|m| m.is_symlink())
            .unwrap_or(false)
    }
}

impl LunchboxPathUtils for Path {}
