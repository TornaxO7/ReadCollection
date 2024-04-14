use std::io::{self, BufRead, BufReader, Read, Seek};

use crate::{RevBufRead, RevRead};

#[derive(Debug)]
pub struct BiBufReader<R> {
    reader: BufReader<R>,
}

impl<R: Read> BiBufReader<R> {
    pub fn buffer(&self) -> &[u8] {
        self.reader.buffer()
    }

    pub fn capacity(&self) -> usize {
        self.reader.capacity()
    }

    pub fn get_mut(&mut self) -> &mut R {
        self.reader.get_mut()
    }

    pub fn get_ref(&self) -> &R {
        self.reader.get_ref()
    }

    pub fn into_inner(self) -> R
    where
        R: Sized,
    {
        self.reader.into_inner()
    }

    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            reader: BufReader::with_capacity(capacity, inner),
        }
    }
}

impl<R: Seek> BiBufReader<R> {
    pub fn seek_relative(&mut self, offset: i64) -> io::Result<()> {
        self.reader.seek_relative(offset)
    }
}

impl<R: Read> Read for BiBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

impl<R: Read> BufRead for BiBufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt)
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
