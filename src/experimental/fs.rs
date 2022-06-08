use std::path::Path;

/// Common interface for file systems. See also the [`fs`] module in [`assets`].
///
/// [`fs`]: crate::assets::fs
/// [`assets`]: crate::assets
pub trait FileSystem {
    /// Reads the given file and calls the given callback with the file's data.
    fn read(&mut self, path: &Path) -> Box<dyn FileTask>;
}

/// Common interface for file loading tasks.
pub trait FileTask {
    /// Polls the file task for completion and returns `None` if the task is not
    /// yet complete or `Some(data)` if it is.
    fn poll(&mut self) -> Option<Vec<u8>>;
}
