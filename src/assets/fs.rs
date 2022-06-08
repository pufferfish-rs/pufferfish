//! File system abstractions.

use std::borrow::Cow;
use std::fs::File as StdFile;
use std::io::Read;
use std::path::Path;

use crate::experimental::{FileSystem, FileTask};

struct BasicFileTask {
    std: StdFile,
    buffer: Option<Vec<u8>>,
    len: usize,
}

impl BasicFileTask {
    fn new(std: StdFile) -> Self {
        let buffer = vec![0; 100000];
        BasicFileTask {
            std,
            buffer: Some(buffer),
            len: 0,
        }
    }
}

impl FileTask for BasicFileTask {
    fn poll(&mut self) -> Option<Vec<u8>> {
        match self
            .std
            .read(&mut self.buffer.as_mut().unwrap()[self.len..])
        {
            Ok(0) => {
                unsafe {
                    self.buffer.as_mut().unwrap().set_len(self.len);
                }
                self.buffer.take()
            }
            Ok(n) => {
                self.len += n;
                None
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => None,
            Err(_) => {
                panic!();
            }
        }
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
        let std = StdFile::open(&path_buf).unwrap();
        let file = BasicFileTask::new(std);
        Box::new(file)
    }
}
