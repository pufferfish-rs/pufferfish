//! File system abstractions.

use std::fs::File as StdFile;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::experimental::{FileSystem, FileTask};

struct BasicFileTask {
    std: StdFile,
    buffer: Vec<u8>,
    len: usize,
    path: PathBuf,
}

impl BasicFileTask {
    fn new(path: PathBuf) -> Self {
        let std = StdFile::open(&path).unwrap();
        let buffer = vec![0; 1024];
        BasicFileTask {
            std,
            buffer,
            len: 0,
            path,
        }
    }
}

impl FileTask for BasicFileTask {
    fn poll(&mut self) -> bool {
        if self.len == self.buffer.len() {
            let mut probe = [0_u8; 1024];
            let len = self.std.read(&mut probe).unwrap();
            if len == 0 {
                return true;
            } else {
                self.buffer.resize(self.len * 2, 0);
                self.buffer[self.len..self.len + len].copy_from_slice(&probe[..len]);
                self.len += len;
            }
        } else {
            match self.std.read(&mut self.buffer[self.len..]) {
                Ok(0) => return true,
                Ok(n) => self.len += n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => {
                    panic!();
                }
            };
        }
        false
    }

    fn data(&self) -> &[u8] {
        &self.buffer[..self.len]
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

/// A file system that simply loads files from the given path.
///
/// By default, assets files are loaded from the parent directory of the
/// executable ([`BasicFileSystem`]) unless the executable is located in a
/// subdirectory of `CARGO_MANIFEST_DIR`, in which case asset files are loaded
/// from `CARGO_MANIFEST_DIR`. All symbolic links are resolved.
pub struct BasicFileSystem {
    root: PathBuf,
}

impl Default for BasicFileSystem {
    fn default() -> Self {
        Self::new_with_root_relative("")
    }
}

impl BasicFileSystem {
    /// Creates a new [`BasicFileSystem`]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new [`BasicFileSystem`] with the given root path.
    pub fn new_with_root(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Creates a new [`BasicFileSystem`] with the given root path.
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

impl FileSystem for BasicFileSystem {
    fn read(&mut self, path: &Path) -> Box<dyn FileTask> {
        let mut path_buf = self.root.clone();
        path_buf.push(path);
        let file = BasicFileTask::new(path_buf);
        Box::new(file)
    }
}
