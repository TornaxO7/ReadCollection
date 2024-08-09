mod buffer;

use crate::{BufReadBack, ReadBack, DEFAULT_BUF_SIZE};

use self::buffer::Buffer;

#[derive(Debug)]
pub struct BufReadBacker<R> {
    inner: R,
    buf: Buffer,
}

impl<R> BufReadBacker<R> {
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn buffer(&self) -> &[u8] {
        self.buf.buffer()
    }

    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn discard_buffer(&mut self) {
        self.buf.discard_buffer();
    }
}

impl<R: ReadBack> BufReadBacker<R> {
    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            inner,
            buf: Buffer::with_capacity(capacity),
        }
    }
}

impl<R: ReadBack> ReadBack for BufReadBacker<R> {
    fn read_back(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.buf.pos() == self.buf.filled() && buf.len() >= self.capacity() {
            self.discard_buffer();
            return self.inner.read_back(buf);
        }

        let mut rem = self.read_back_fill_buf()?;
        let nread = rem.read_back(buf)?;
        self.read_back_consume(nread);
        Ok(nread)
    }
}

impl<R: ReadBack> BufReadBack for BufReadBacker<R> {
    fn read_back_fill_buf(&mut self) -> std::io::Result<&[u8]> {
        self.buf.fill_buf(&mut self.inner)
    }

    fn read_back_consume(&mut self, amt: usize) {
        self.buf.consume(amt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general() {
        let data: [u8; 3] = [1, 2, 3];
        let mut buffer: [u8; 3] = [0; 3];

        let mut buf_reader = BufReadBacker::new(data.as_slice());

        assert_eq!(buf_reader.read_back(&mut buffer).ok(), Some(data.len()));
        assert_eq!(buffer, data);
    }

    #[test]
    fn small_capacity() {
        let data: [u8; 4] = [1, 2, 3, 4];
        let mut buffer: [u8; 2] = [0; 2];

        let mut buf_reader = BufReadBacker::with_capacity(2, data.as_slice());

        assert_eq!(buf_reader.read_back(&mut buffer).ok(), Some(2));
        assert_eq!(buffer, data[2..4]);

        assert_eq!(buf_reader.read_back(&mut buffer).ok(), Some(2));
        assert_eq!(buffer, data[..2]);
    }
}
