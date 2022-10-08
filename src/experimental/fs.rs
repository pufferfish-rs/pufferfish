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
    /// Polls the file task for completion.
    fn poll(&mut self) -> bool;

    /// Returns a slice of the data buffer. The buffer may not be fully
    /// populated if the task is not yet complete.
    fn data(&self) -> &[u8];

    /// Returns the extension of the file being loaded.
    fn extension(&self) -> &str;
}
