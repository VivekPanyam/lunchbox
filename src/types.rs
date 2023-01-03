// Note: the vast majority of docs in this file are heavily derived from the `tokio::fs` and `std::fs` docs
use async_trait::async_trait;
use std::{
    fs::FileType,
    future::poll_fn,
    io::Result,
    task::{Context, Poll},
    time::SystemTime,
};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::path::{Path, PathBuf};

/// All lunchbox filesystems implement this trait.
/// In order to be useful, `FileType` should implement
/// [`ReadableFile`] and possibly [`WritableFile`]
pub trait HasFileType {
    type FileType;
}

/// PathType is an alias for `AsRef<Path> + Send`
pub trait PathType: AsRef<Path> + Send {}
impl<T: AsRef<Path> + Send + ?Sized> PathType for T {}

/// A readable filesystem
#[async_trait]
pub trait ReadableFileSystem: HasFileType
where
    Self: Sized,
{
    /// Equivalent to
    /// ```
    /// async fn open(&self, path: impl PathType) -> Result<Self::FileType>
    /// where
    ///     Self::FileType: ReadableFile;
    /// ```
    /// Attempts to open a file in read-only mode.
    ///
    /// See [`OpenOptions`] for more details.
    async fn open(&self, path: impl PathType) -> Result<Self::FileType>
    where
        Self::FileType: ReadableFile;

    // The following are directly based on `tokio::fs::...`

    /// Equivalent to
    /// ```
    /// async fn canonicalize(&self, path: impl PathType) -> Result<PathBuf>;
    /// ```
    /// Returns the canonical, absolute form of a path with all intermediate
    /// components normalized and symbolic links resolved.
    ///
    /// This is an async version of [`std::fs::canonicalize`]
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * `path` does not exist.
    /// * A non-final component in path is not a directory.
    ///
    async fn canonicalize(&self, path: impl PathType) -> Result<PathBuf>;

    /// Equivalent to
    /// ```
    /// async fn metadata(&self, path: impl PathType) -> Result<Metadata>;
    /// ```
    /// Given a path, queries the file system to get information about a file,
    /// directory, etc.
    ///
    /// This is an async version of [`std::fs::metadata`]
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * The user lacks permissions to perform `metadata` call on `path`.
    /// * `path` does not exist.
    ///
    async fn metadata(&self, path: impl PathType) -> Result<Metadata>;

    /// Equivalent to
    /// ```
    /// async fn read(&self, path: impl PathType) -> Result<Vec<u8>>;
    /// ```
    /// Reads the entire contents of a file into a bytes vector.
    ///
    /// This is an async version of [`std::fs::read`]
    ///
    ///
    /// This is a convenience function for using [`ReadableFileSystem::open`] and `tokio::io::AsyncReadExt::read_to_end`
    /// with fewer imports and without an intermediate variable. It pre-allocates a
    /// buffer based on the file size when available, so it is generally faster than
    /// reading into a vector created with `Vec::new()`.
    ///
    /// # Errors
    ///
    /// This function will return an error if `path` does not already exist.
    /// Other errors may also be returned according to [`OpenOptions::open`].
    ///
    /// [`OpenOptions::open`]: OpenOptions::open
    ///
    /// It will also return an error if it encounters while reading an error
    /// of a kind other than [`ErrorKind::Interrupted`].
    ///
    /// [`ErrorKind::Interrupted`]: std::io::ErrorKind::Interrupted
    async fn read(&self, path: impl PathType) -> Result<Vec<u8>>;

    type ReadDirPollerType: ReadDirPoller<Self>;

    /// Equivalent to
    /// ```
    /// async fn read_dir(&self, path: impl PathType)
    ///     -> Result<ReadDir<Self::ReadDirPollerType, Self>>;
    /// ```
    /// Returns a stream over the entries within a directory.
    ///
    /// This is an async version of [`std::fs::read_dir`]
    ///
    /// See [`ReadDir`] for more details and examples.
    async fn read_dir(&self, path: impl PathType)
        -> Result<ReadDir<Self::ReadDirPollerType, Self>>;

    /// Equivalent to
    /// ```
    /// async fn read_link(&self, path: impl PathType) -> Result<PathBuf>;
    /// ```
    /// Reads a symbolic link, returning the file that the link points to.
    ///
    /// This is an async version of [`std::fs::read_link`]
    async fn read_link(&self, path: impl PathType) -> Result<PathBuf>;

    /// Equivalent to
    /// ```
    /// async fn read_to_string(&self, path: impl PathType) -> Result<String>;
    /// ```
    /// Creates a future which will open a file for reading and read the entire
    /// contents into a string and return said string.
    ///
    /// This is the async equivalent of [`std::fs::read_to_string`]
    async fn read_to_string(&self, path: impl PathType) -> Result<String>;

    /// Equivalent to
    /// ```
    /// async fn symlink_metadata(&self, path: impl PathType) -> Result<Metadata>;
    /// ```
    /// Queries the file system metadata for a path.
    ///
    /// This is an async version of [`std::fs::symlink_metadata`]
    async fn symlink_metadata(&self, path: impl PathType) -> Result<Metadata>;
}

/// A writable filesystem
#[async_trait]
pub trait WritableFileSystem: ReadableFileSystem {
    /// Equivalent to
    /// ```
    /// async fn create(&self, path: impl PathType) -> Result<Self::FileType>
    /// where
    ///     Self::FileType: WritableFile;
    /// ```
    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will truncate
    /// it if it does.
    ///
    /// See [`OpenOptions`] for more details.
    async fn create(&self, path: impl PathType) -> Result<Self::FileType>
    where
        Self::FileType: WritableFile,
    {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self, path)
            .await
    }

    /// Equivalent to
    /// ```
    /// async fn open_with_opts(
    ///     &self,
    ///     opts: &OpenOptions,
    ///     path: impl PathType,
    /// ) -> Result<Self::FileType>
    /// where
    ///     Self::FileType: WritableFile;
    /// ```
    /// Used by [`OpenOptions::open`]
    async fn open_with_opts(
        &self,
        opts: &OpenOptions,
        path: impl PathType,
    ) -> Result<Self::FileType>
    where
        Self::FileType: WritableFile;

    // The following are directly based on `tokio::fs::...`

    /// Equivalent to
    /// ```
    /// async fn copy(&self, from: impl PathType, to: impl PathType) -> Result<u64>;
    /// ```
    /// Copies the contents of one file to another. This function will also copy the permission bits
    /// of the original file to the destination file.
    /// This function will overwrite the contents of `to`.
    ///
    /// This is the async equivalent of [`std::fs::copy`]
    async fn copy(&self, from: impl PathType, to: impl PathType) -> Result<u64>;

    /// Equivalent to
    /// ```
    /// async fn create_dir(&self, path: impl PathType) -> Result<()>;
    /// ```
    /// Creates a new, empty directory at the provided path.
    ///
    /// This is an async version of [`std::fs::create_dir`]
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * User lacks permissions to create directory at `path`.
    /// * A parent of the given path doesn't exist. (To create a directory and all
    ///   its missing parents at the same time, use the [`create_dir_all`]
    ///   function.)
    /// * `path` already exists.
    ///
    /// [`create_dir_all`]: Self::create_dir_all
    async fn create_dir(&self, path: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn create_dir_all(&self, path: impl PathType) -> Result<()>;
    /// ```
    /// Recursively creates a directory and all of its parent components if they
    /// are missing.
    ///
    /// This is an async version of [`std::fs::create_dir_all`]
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * If any directory in the path specified by `path` does not already exist
    /// and it could not be created otherwise. The specific error conditions for
    /// when a directory is being created (after it is determined to not exist) are
    /// outlined by [`fs::create_dir`].
    ///
    /// Notable exception is made for situations where any of the directories
    /// specified in the `path` could not be created as it was being created concurrently.
    /// Such cases are considered to be successful. That is, calling `create_dir_all`
    /// concurrently from multiple threads or processes is guaranteed not to fail
    /// due to a race condition with itself.
    ///
    /// [`fs::create_dir`]: std::fs::create_dir
    async fn create_dir_all(&self, path: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn hard_link(&self, src: impl PathType, dst: impl PathType) -> Result<()>;
    /// ```
    /// Creates a new hard link on the filesystem.
    ///
    /// This is an async version of [`std::fs::hard_link`]
    ///
    /// The `dst` path will be a link pointing to the `src` path.
    ///
    /// # Errors
    ///
    /// This function will return an error in the following situations, but is not
    /// limited to just these cases:
    ///
    /// * The `src` path is not a file or doesn't exist.
    async fn hard_link(&self, src: impl PathType, dst: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn remove_dir(&self, path: impl PathType) -> Result<()>;
    /// ```
    /// Removes an existing, empty directory.
    ///
    /// This is an async version of [`std::fs::remove_dir`]
    async fn remove_dir(&self, path: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn remove_dir_all(&self, path: impl PathType) -> Result<()>;
    /// ```
    /// Removes a directory at this path, after removing all its contents. Use carefully!
    ///
    /// This is an async version of [`std::fs::remove_dir_all`]
    async fn remove_dir_all(&self, path: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn remove_file(&self, path: impl PathType) -> Result<()>;
    /// ```
    /// Removes a file from the filesystem.
    ///
    /// Note that there is no guarantee that the file is immediately deleted (e.g.
    /// depending on platform, other open file descriptors may prevent immediate
    /// removal).
    ///
    /// This is an async version of [`std::fs::remove_file`]
    async fn remove_file(&self, path: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn rename(&self, from: impl PathType, to: impl PathType) -> Result<()>;
    /// ```
    /// Renames a file or directory to a new name, replacing the original file if
    /// `to` already exists.
    ///
    /// This is an async version of [`std::fs::rename`]
    async fn rename(&self, from: impl PathType, to: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn set_permissions(&self, path: impl PathType, perm: Permissions) -> Result<()>;
    /// ```
    /// Changes the permissions found on a file or a directory.
    ///
    /// Filesystem implementations may or may not support this.
    async fn set_permissions(&self, path: impl PathType, perm: Permissions) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn symlink(&self, src: impl PathType, dst: impl PathType) -> Result<()>;
    /// ```
    /// Creates a new symbolic link on the filesystem.
    ///
    /// The `dst` path will be a symbolic link pointing to the `src` path.
    ///
    /// This is an async version of [`std::os::unix::fs::symlink`]
    async fn symlink(&self, src: impl PathType, dst: impl PathType) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn write(&self, path: impl PathType, contents: impl AsRef<[u8]> + Send) -> Result<()>;
    /// ```
    /// Creates a future that will open a file for writing and write the entire
    /// contents of `contents` to it.
    ///
    /// This is the async equivalent of [`std::fs::write`]
    async fn write(&self, path: impl PathType, contents: impl AsRef<[u8]> + Send) -> Result<()>;
}

/// A readable file (that must also implement [`AsyncRead`])
#[async_trait]
pub trait ReadableFile: AsyncRead
where
    Self: Sized,
{
    /// Equivalent to
    /// ```
    /// async fn metadata(&self) -> Result<Metadata>;
    /// ```
    /// Queries metadata about the underlying file.
    async fn metadata(&self) -> Result<Metadata>;

    /// Equivalent to
    /// ```
    /// async fn try_clone(&self) -> Result<Self>;
    /// ```
    /// Creates a new `File` instance that shares the same underlying file handle
    /// as the existing `File` instance. Reads, writes, and seeks will affect both
    /// File instances simultaneously.
    async fn try_clone(&self) -> Result<Self>;
}

/// A file that supports both reads and writes
/// Must also implement [`ReadableFile`] and [`AsyncWrite`]
#[async_trait]
pub trait WritableFile: ReadableFile + AsyncWrite {
    /// Equivalent to
    /// ```
    /// async fn sync_all(&self) -> Result<()>;
    /// ```
    /// Attempts to sync all OS-internal metadata to disk.
    ///
    /// This function will attempt to ensure that all in-core data reaches the
    /// filesystem before returning.
    async fn sync_all(&self) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn sync_data(&self) -> Result<()>;
    /// ```
    /// This function is similar to `sync_all`, except that it may not
    /// synchronize file metadata to the filesystem.
    ///
    /// This is intended for use cases that must synchronize content, but don't
    /// need the metadata on disk. The goal of this method is to reduce disk
    /// operations.
    ///
    /// Note that some platforms may simply implement this in terms of `sync_all`.
    async fn sync_data(&self) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn set_len(&self, size: u64) -> Result<()>;
    /// ```
    /// Truncates or extends the underlying file, updating the size of this file to become size.
    ///
    /// If the size is less than the current file's size, then the file will be
    /// shrunk. If it is greater than the current file's size, then the file
    /// will be extended to size and have all of the intermediate data filled in
    /// with 0s.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file is not opened for
    /// writing.
    async fn set_len(&self, size: u64) -> Result<()>;

    /// Equivalent to
    /// ```
    /// async fn set_permissions(&self, perm: Permissions) -> Result<()>;
    /// ```
    /// Changes the permissions on the underlying file.
    /// See the docs for individual filesystem implementations to see how they handle permissions
    ///
    /// # Errors
    ///
    /// This function will return an error if the user lacks permission change
    /// attributes on the underlying file. It may also return an error in other
    /// unspecified cases.
    async fn set_permissions(&self, perm: Permissions) -> Result<()>;
}

/// Entries returned by a [`ReadDir`] stream.
///
/// An instance of `DirEntry` represents an entry inside of a directory on a
/// filesystem. Each entry can be inspected via methods to learn about the full
/// path.
pub struct DirEntry<'a, F> {
    fs: &'a F,
    file_name: String,
    path: PathBuf,
}

impl<'a, F: ReadableFileSystem> DirEntry<'a, F>
where
    F::FileType: ReadableFile,
{
    pub fn new(fs: &'a F, file_name: String, path: PathBuf) -> DirEntry<F> {
        DirEntry {
            fs,
            file_name,
            path,
        }
    }

    /// Returns the bare file name of this directory entry without any other
    /// leading path component.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut entries = fs.read_dir(".").await?;
    ///
    /// while let Some(entry) = entries.next_entry().await? {
    ///     println!("{:?}", entry.file_name());
    /// }
    /// ```
    pub fn file_name(&self) -> String {
        self.file_name.clone()
    }

    /// Returns the full path to the file that this entry represents.
    ///
    /// The full path may be created by joining the original path to `read_dir`
    /// with the filename of this entry. Therefore it may not be an absolute
    /// path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut entries = fs.read_dir(".").await?;
    ///
    /// while let Some(entry) = entries.next_entry().await? {
    ///     println!("{:?}", entry.path());
    /// }
    /// ```
    ///
    /// This prints output like:
    ///
    /// ```text
    /// "./whatever.txt"
    /// "./foo.html"
    /// "./hello_world.rs"
    /// ```
    ///
    /// The exact text, of course, depends on what files you have in `.`.
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Returns the file type for the file that this entry points at.
    ///
    /// This function will not traverse symlinks if this entry points at a
    /// symlink.
    ///
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut entries = fs.read_dir(".").await?;
    ///
    /// while let Some(entry) = entries.next_entry().await? {
    ///     if let Ok(file_type) = entry.file_type().await {
    ///         // Now let's show our entry's file type!
    ///         println!("{:?}: {:?}", entry.path(), file_type);
    ///     } else {
    ///         println!("Couldn't get file type for {:?}", entry.path());
    ///     }
    /// }
    /// ```
    pub async fn file_type(&self) -> Result<FileType> {
        Ok(self.metadata().await?.file_type())
    }

    /// Returns the metadata for the file that this entry points at.
    ///
    /// This function will not traverse symlinks if this entry points at a
    /// symlink.
    ///
    /// This function is the equivalent of calling `symlink_metadata` on the path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut entries = fs.read_dir(".").await?;
    ///
    /// while let Some(entry) = entries.next_entry().await? {
    ///     if let Ok(metadata) = entry.metadata().await {
    ///         // Now let's show our entry's permissions!
    ///         println!("{:?}: {:?}", entry.path(), metadata.permissions());
    ///     } else {
    ///         println!("Couldn't get file type for {:?}", entry.path());
    ///     }
    /// }
    /// ```
    pub async fn metadata(&self) -> Result<Metadata> {
        // This is equivalent to calling symlink_metadata
        // See https://doc.rust-lang.org/std/fs/struct.DirEntry.html#method.metadata
        self.fs.symlink_metadata(&self.path).await
    }
}

/// Used when implementing a lunchbox filesystem.
/// Users of lunchbox filesystems can ignore this trait
pub trait ReadDirPoller<F: ReadableFileSystem> {
    /// Polls for the next directory entry in the stream.
    ///
    /// This method returns:
    ///
    ///  * `Poll::Pending` if the next directory entry is not yet available.
    ///  * `Poll::Ready(Ok(Some(entry)))` if the next directory entry is available.
    ///  * `Poll::Ready(Ok(None))` if there are no more directory entries in this
    ///    stream.
    ///  * `Poll::Ready(Err(err))` if an IO error occurred while reading the next
    ///    directory entry.
    ///
    /// When the method returns `Poll::Pending`, the `Waker` in the provided
    /// `Context` should be scheduled to receive a wakeup when the next directory entry
    /// becomes available on the underlying IO resource.
    ///
    /// Note that on multiple calls to `poll_next_entry`, only the `Waker` from
    /// the `Context` passed to the most recent call should be scheduled to receive a
    /// wakeup.
    fn poll_next_entry<'a>(
        &mut self,
        cx: &mut Context<'_>,
        fs: &'a F,
    ) -> Poll<std::io::Result<Option<DirEntry<'a, F>>>>;
}

/// Reads the entries in a directory.
///
/// This struct is returned from the [`read_dir`] method and
/// will yield instances of [`DirEntry`]. Through a [`DirEntry`], information
/// like the entry's path and possibly other metadata can be learned.
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
///
/// # Errors
///
/// This stream will return an [`Err`] if there's some sort of intermittent
/// IO error during iteration.
///
/// [`read_dir`]: ReadableFileSystem::read_dir
/// [`DirEntry`]: DirEntry
/// [`Err`]: std::result::Result::Err
pub struct ReadDir<'a, T, F> {
    poller: T,
    fs: &'a F,
}

impl<'a, T: ReadDirPoller<F>, F: ReadableFileSystem> ReadDir<'a, T, F> {
    /// Create a new `ReadDir` with a [`ReadDirPoller`]
    pub fn new(poller: T, fs: &'a F) -> ReadDir<'a, T, F> {
        ReadDir { poller, fs }
    }

    /// Returns the next entry in the directory stream.
    pub async fn next_entry(&mut self) -> Result<Option<DirEntry<F>>> {
        poll_fn(|cx| self.poller.poll_next_entry(cx, self.fs)).await
    }
}

/// Metadata information about a file.
///
/// This structure is returned from the [`metadata`](ReadableFileSystem::metadata) or
/// [`symlink_metadata`](ReadableFileSystem::symlink_metadata) methods and represents known
/// metadata about a file such as its permissions, size, modification
/// times, etc.
#[derive(Debug)]
pub struct Metadata {
    accessed: Result<SystemTime>,
    created: Result<SystemTime>,
    modified: Result<SystemTime>,
    file_type: FileType,
    len: u64,
    permissions: Permissions,
}

impl Metadata {
    pub fn new(
        accessed: Result<SystemTime>,
        created: Result<SystemTime>,
        modified: Result<SystemTime>,
        file_type: FileType,
        len: u64,
        permissions: Permissions,
    ) -> Metadata {
        Metadata {
            accessed,
            created,
            modified,
            file_type,
            len,
            permissions,
        }
    }

    /// Returns the last access time of this metadata.
    ///
    /// Note that not all platforms will keep this field update in a file's
    /// metadata.
    ///
    /// # Errors
    ///
    /// This field might not be available on all platforms, and will return an
    /// `Err` on platforms where it is not available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// if let Ok(time) = metadata.accessed() {
    ///     println!("{time:?}");
    /// } else {
    ///     println!("Not supported on this platform");
    /// }
    /// ```
    pub fn accessed(&self) -> Result<SystemTime> {
        match &self.accessed {
            Ok(item) => Ok(item.clone()),
            Err(e) => Err(std::io::Error::new(e.kind(), e.to_string())),
        }
    }

    /// Returns the creation time listed in this metadata.
    ///
    ///
    /// # Errors
    ///
    /// This field might not be available on all platforms, and will return an
    /// `Err` on platforms or filesystems where it is not available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// if let Ok(time) = metadata.created() {
    ///     println!("{time:?}");
    /// } else {
    ///     println!("Not supported on this platform or filesystem");
    /// }
    /// ```
    pub fn created(&self) -> Result<SystemTime> {
        match &self.created {
            Ok(item) => Ok(item.clone()),
            Err(e) => Err(std::io::Error::new(e.kind(), e.to_string())),
        }
    }

    /// Returns the last modification time listed in this metadata.
    ///
    ///
    /// # Errors
    ///
    /// This field might not be available on all platforms, and will return an
    /// `Err` on platforms where it is not available.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// if let Ok(time) = metadata.modified() {
    ///     println!("{time:?}");
    /// } else {
    ///     println!("Not supported on this platform");
    /// }
    /// ```
    pub fn modified(&self) -> Result<SystemTime> {
        match &self.modified {
            Ok(item) => Ok(item.clone()),
            Err(e) => Err(std::io::Error::new(e.kind(), e.to_string())),
        }
    }

    /// Returns the file type for this metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// println!("{:?}", metadata.file_type());
    /// ```
    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Returns `true` if this metadata is for a directory. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_file`], and will be false for symlink metadata
    /// obtained from [`symlink_metadata`](ReadableFileSystem::symlink_metadata).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// assert!(!metadata.is_dir());
    /// ```
    pub fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    /// Returns `true` if this metadata is for a regular file. The
    /// result is mutually exclusive to the result of
    /// [`Metadata::is_dir`], and will be false for symlink metadata
    /// obtained from [`symlink_metadata`](ReadableFileSystem::symlink_metadata).
    ///
    /// When the goal is simply to read from (or write to) the source, the most
    /// reliable way to test the source can be read (or written to) is to open
    /// it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// assert!(metadata.is_file());
    /// ```
    pub fn is_file(&self) -> bool {
        self.file_type().is_file()
    }

    /// Returns `true` if this metadata is for a symbolic link.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::path::Path;
    ///
    /// let link_path = Path::new("link");
    /// let metadata = fs.symlink_metadata(link_path).await?;
    /// println!("{:#?}", metadata.is_symlink());
    /// ```
    pub fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }

    /// Returns the size of the file, in bytes, this metadata is for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// assert_eq!(0, metadata.len());
    /// ```
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Returns the permissions of the file this metadata is for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let metadata = fs.metadata("foo.txt").await?;
    ///
    /// assert!(!metadata.permissions().readonly());
    /// ```
    pub fn permissions(&self) -> Permissions {
        self.permissions.clone()
    }
}

impl From<std::fs::Metadata> for Metadata {
    fn from(metadata: std::fs::Metadata) -> Self {
        Metadata::new(
            metadata.accessed(),
            metadata.created(),
            metadata.modified(),
            metadata.file_type(),
            metadata.len(),
            metadata.permissions().into(),
        )
    }
}

/// Representation of the various permissions on a file.
///
/// This module only currently provides one bit of information,
/// [`Permissions::readonly`]
#[derive(Debug, Clone)]
pub struct Permissions {
    ro: bool,
}

impl Permissions {
    pub fn new(readonly: bool) -> Permissions {
        Permissions { ro: readonly }
    }

    /// Returns true if these permissions describe a readonly (unwritable) file.
    ///
    /// See the docs for individual filesystem implementations to see how they handle permissions
    pub fn readonly(&self) -> bool {
        self.ro
    }

    /// Modifies the readonly flag for this set of permissions. If the
    /// `readonly` argument is `true`, using the resulting `Permission` will
    /// update file permissions to forbid writing. Conversely, if it's `false`,
    /// using the resulting `Permission` will update file permissions to allow
    /// writing.
    ///
    /// This operation does **not** modify the files attributes. This only
    /// changes the in-memory value of these attributes for this `Permissions`
    /// instance. To modify the files attributes use the [`set_permissions`](WritableFileSystem::set_permissions)
    /// method which commits these attribute changes to the file.
    ///
    /// See the docs for individual filesystem implementations to see how they handle permissions
    pub fn set_readonly(&mut self, readonly: bool) {
        self.ro = readonly
    }
}

impl From<std::fs::Permissions> for Permissions {
    fn from(value: std::fs::Permissions) -> Self {
        Permissions {
            ro: value.readonly(),
        }
    }
}

// Mostly copied from `std::fs::OpenOptions`
/// Options and flags which can be used to configure how a writable file is opened.
///
/// For a [`WritableFileSystem`] `T`, This builder exposes the ability to configure how a
/// `T::FileType` (the filetype of the filesystem) is opened and what operations
/// are permitted on the open file. The [`WritableFileSystem::create`] method is an alias
/// for commonly used options using this builder.
///
/// Generally speaking, when using `OpenOptions`, you'll first call [`OpenOptions::new`],
/// then chain calls to methods to set each option, then call [`open`](OpenOptions::open), passing
/// the path of the file you're trying to open. This will give you a
/// [`io::Result`][std::io::Result] with a `T::FileType` inside that you can further operate
/// on.
///
/// Please note that [`OpenOptions`] are only available for [`WritableFileSystem`]s
#[derive(Clone, Debug)]
pub struct OpenOptions {
    inner: OpenOptionsInner,
}

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct OpenOptionsInner {
    pub read: bool,
    pub write: bool,
    pub append: bool,
    pub truncate: bool,
    pub create: bool,
    pub create_new: bool,
}

// Mostly copied from `std::fs::OpenOptions`
impl OpenOptions {
    /// Creates a blank new set of options ready for configuration.
    ///
    /// All options are initially set to `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let mut options = OpenOptions::new();
    /// let file = options.read(true).open(fs, "foo.txt");
    /// ```
    #[must_use]
    pub fn new() -> OpenOptions {
        let inner = OpenOptionsInner {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        };

        OpenOptions { inner }
    }

    /// Sets the option for read access.
    ///
    /// This option, when true, will indicate that the file should be
    /// `read`-able if opened.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let file = OpenOptions::new().read(true).open(fs, "foo.txt");
    /// ```
    pub fn read(&mut self, read: bool) -> &mut Self {
        self.inner.read = read;
        self
    }

    /// Sets the option for write access.
    ///
    /// This option, when true, will indicate that the file should be
    /// `write`-able if opened.
    ///
    /// If the file already exists, any write calls on it will overwrite its
    /// contents, without truncating it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let file = OpenOptions::new().write(true).open(fs, "foo.txt");
    /// ```
    pub fn write(&mut self, write: bool) -> &mut Self {
        self.inner.write = write;
        self
    }

    /// Sets the option for the append mode.
    ///
    /// This option, when true, means that writes will append to a file instead
    /// of overwriting previous contents.
    /// Note that setting `.write(true).append(true)` has the same effect as
    /// setting only `.append(true)`.
    ///
    /// For most filesystems, the operating system guarantees that all writes are
    /// atomic: no writes get mangled because another process writes at the same
    /// time.
    ///
    /// One maybe obvious note when using append-mode: make sure that all data
    /// that belongs together is written to the file in one operation. This
    /// can be done by concatenating strings before passing them to [`write()`],
    /// or using a buffered writer (with a buffer of adequate size),
    /// and calling [`flush()`] when the message is complete.
    ///
    /// If a file is opened with both read and append access, beware that after
    /// opening, and after every write, the position for reading may be set at the
    /// end of the file. So, before writing, save the current position (using
    /// <code>[seek]\([std::io::SeekFrom]::[Current]\(0))</code>), and restore it before the next read.
    ///
    /// ## Note
    ///
    /// This function doesn't create the file if it doesn't exist. Use the
    /// [`OpenOptions::create`] method to do so.
    ///
    /// [`write()`]: std::io::Write::write
    /// [`flush()`]: std::io::Write::flush
    /// [seek]: std::io::Seek::seek
    /// [Current]: std::io::SeekFrom::Current
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let file = OpenOptions::new().append(true).open(fs, "foo.txt");
    /// ```
    pub fn append(&mut self, append: bool) -> &mut Self {
        self.inner.append = append;
        self
    }

    /// Sets the option for truncating a previous file.
    ///
    /// If a file is successfully opened with this option set it will truncate
    /// the file to 0 length if it already exists.
    ///
    /// The file must be opened with write access for truncate to work.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let file = OpenOptions::new().write(true).truncate(true).open(fs, "foo.txt");
    /// ```
    pub fn truncate(&mut self, truncate: bool) -> &mut Self {
        self.inner.truncate = truncate;
        self
    }

    /// Sets the option to create a new file, or open it if it already exists.
    ///
    /// In order for the file to be created, [`OpenOptions::write`] or
    /// [`OpenOptions::append`] access must be used.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let file = OpenOptions::new().write(true).create(true).open(fs, "foo.txt");
    /// ```
    pub fn create(&mut self, create: bool) -> &mut Self {
        self.inner.create = create;
        self
    }

    /// Sets the option to create a new file, failing if it already exists.
    ///
    /// No file is allowed to exist at the target location, also no (dangling) symlink. In this
    /// way, if the call succeeds, the file returned is guaranteed to be new.
    ///
    /// This option is useful because it is atomic. Otherwise between checking
    /// whether a file exists and creating a new one, the file may have been
    /// created by another process (a TOCTOU race condition / attack).
    ///
    /// If `.create_new(true)` is set, [`.create()`] and [`.truncate()`] are
    /// ignored.
    ///
    /// The file must be opened with write or append access in order to create
    /// a new file.
    ///
    /// [`.create()`]: OpenOptions::create
    /// [`.truncate()`]: OpenOptions::truncate
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let file = OpenOptions::new().write(true)
    ///                              .create_new(true)
    ///                              .open(fs, "foo.txt");
    /// ```
    pub fn create_new(&mut self, create_new: bool) -> &mut Self {
        self.inner.create_new = create_new;
        self
    }

    /// Opens a file at `path` with the options specified by `self`.
    ///
    /// # Errors
    ///
    /// This function will return an error under a number of different
    /// circumstances. Some of these error conditions are listed here, together
    /// with their [`std::io::ErrorKind`]. The mapping to [`std::io::ErrorKind`]s is not
    /// part of the compatibility contract of the function.
    ///
    /// * [`NotFound`]: The specified file does not exist and neither `create`
    ///   or `create_new` is set.
    /// * [`NotFound`]: One of the directory components of the file path does
    ///   not exist.
    /// * [`PermissionDenied`]: The user lacks permission to get the specified
    ///   access rights for the file.
    /// * [`PermissionDenied`]: The user lacks permission to open one of the
    ///   directory components of the specified path.
    /// * [`AlreadyExists`]: `create_new` was specified and the file already
    ///   exists.
    /// * [`InvalidInput`]: Invalid combinations of open options (truncate
    ///   without write access, no access mode set, etc.).
    ///
    /// The following errors don't match any existing [`std::io::ErrorKind`] at the moment:
    /// * One of the directory components of the specified file path
    ///   was not, in fact, a directory.
    /// * Filesystem-level errors: full disk, write permission
    ///   requested on a read-only file system, exceeded disk quota, too many
    ///   open files, too long filename, too many symbolic links in the
    ///   specified path (Unix-like systems only), etc.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lunchbox::types::OpenOptions;
    ///
    /// let file = OpenOptions::new().read(true).open(fs, "foo.txt");
    /// ```
    ///
    /// [`AlreadyExists`]: std::io::ErrorKind::AlreadyExists
    /// [`InvalidInput`]: std::io::ErrorKind::InvalidInput
    /// [`NotFound`]: std::io::ErrorKind::NotFound
    /// [`PermissionDenied`]: std::io::ErrorKind::PermissionDenied
    pub async fn open<T: WritableFileSystem>(
        &self,
        fs: &T,
        path: impl PathType,
    ) -> std::io::Result<T::FileType>
    where
        T::FileType: WritableFile,
    {
        fs.open_with_opts(self, path).await
    }
}

impl From<&OpenOptions> for OpenOptionsInner {
    fn from(value: &OpenOptions) -> Self {
        value.inner.clone()
    }
}
