//! File system abstractions.

use std::fs::File as StdFile;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;

use crate::experimental::{FileSystem, FileTask};

struct BasicFileTask {
    thread: Option<JoinHandle<Vec<u8>>>,
    buffer: Vec<u8>,
    path: PathBuf,
}

impl BasicFileTask {
    fn new(path: PathBuf) -> Self {
        let path2 = path.clone();
        let thread = Some(std::thread::spawn(move || {
            let mut std = StdFile::open(path2).unwrap();
            let mut buffer = Vec::new();
            std.read_to_end(&mut buffer).unwrap();
            buffer
        }));
        let buffer = Vec::new();
        BasicFileTask {
            thread,
            buffer,
            path,
        }
    }
}

impl FileTask for BasicFileTask {
    fn poll(&mut self) -> bool {
        if self.thread.is_none() {
            true
        } else if self.thread.as_ref().unwrap().is_finished() {
            self.buffer = self.thread.take().unwrap().join().unwrap();
            true
        } else {
            false
        }
    }

    fn data(&self) -> &[u8] {
        &self.buffer
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

/// A file system that loads files synchronously in a different thread.
///
/// By default, [`ThreadedFileSystem`] loads asset files relative to the parent
/// directory of the executable unless the executable is located in a
/// subdirectory of `CARGO_MANIFEST_DIR`, in which case asset files are loaded
/// from `CARGO_MANIFEST_DIR`. All symbolic links are resolved.
pub struct ThreadedFileSystem {
    root: PathBuf,
}

impl Default for ThreadedFileSystem {
    fn default() -> Self {
        Self::new_with_root_relative("")
    }
}

impl ThreadedFileSystem {
    /// Creates a new [`ThreadedFileSystem`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new [`ThreadedFileSystem`] with the given root path.
    pub fn new_with_root(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Creates a new [`ThreadedFileSystem`] with the given root path.
    ///
    /// The root path is interpreted relatively to the default root
    /// path.
    pub fn new_with_root_relative(root: impl AsRef<Path>) -> Self {
        let mut default_root = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().and_then(|e| e.canonicalize().ok()))
            .map(|e| {
                std::env::var("CARGO_MANIFEST_DIR")
                    .ok()
                    .and_then(|e| Path::new(&e).canonicalize().ok())
                    .and_then(|p| e.starts_with(&p).then(|| p))
                    .unwrap_or(e)
            })
            .unwrap_or_else(PathBuf::new);
        default_root.push(root);
        Self::new_with_root(default_root)
    }
}

impl FileSystem for ThreadedFileSystem {
    fn read(&mut self, path: &Path) -> Box<dyn FileTask> {
        let mut path_buf = self.root.clone();
        path_buf.push(path);
        let file = BasicFileTask::new(path_buf);
        Box::new(file)
    }
}
