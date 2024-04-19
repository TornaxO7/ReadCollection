use std::io::{self, BufRead, Read, Seek};

use crate::{RevBufRead, RevRead};

use self::buffer::Buffer;

mod buffer;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
pub const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

pub struct BiBufReader<R> {
    buf: buffer::Buffer,
    inner: R,
}

impl<R: Read> BiBufReader<R> {
    pub fn buffer(&self) -> &[u8] {
        self.buf.buffer()
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn into_inner(self) -> R
    where
        R: Sized,
    {
        self.inner
    }

    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            buf: Buffer::with_capacity(capacity),
            inner,
        }
    }
}

impl<R: Seek> BiBufReader<R> {
    pub fn seek_relative(&mut self, offset: i64) -> io::Result<()> {
        todo!()
    }
}

impl<R: Read> Read for BiBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

impl<R: Read> BufRead for BiBufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        todo!()
    }

    fn consume(&mut self, amt: usize) {
        todo!()
    }
}

impl<R: Seek> RevRead for BiBufReader<R> {
    fn rev_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

impl<R: Seek> RevBufRead for BiBufReader<R> {
    fn rev_fill_buf(&mut self) -> io::Result<&[u8]> {
        todo!()
    }

    fn rev_consume(&mut self, amt: usize) {
        todo!()
    }
}
