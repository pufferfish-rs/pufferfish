//! File system abstractions.

use std::borrow::Cow;
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
                println!("Resizing buffer from {} bytes to {} bytes; {} new bytes read", self.len, self.len * 2, len);
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
pub struct BasicFileSystem {
    base_path: Cow<'static, Path>,
}

impl BasicFileSystem {
    /// Creates a new basic file system with the given base path.
    pub fn new(base_path: impl Into<Cow<'static, Path>>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }
}

impl FileSystem for BasicFileSystem {
    fn read(&mut self, path: &Path) -> Box<dyn FileTask> {
        let mut path_buf = self.base_path.to_path_buf();
        path_buf.push(path);
        let file = BasicFileTask::new(path_buf);
        Box::new(file)
    }
}
