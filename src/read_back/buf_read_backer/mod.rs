mod buffer;

use std::io::BufReader;

use crate::{BufReadBack, ReadBack, DEFAULT_BUF_SIZE};

use self::buffer::Buffer;

/// The `BufReadBacker<R>` struct adds buffering to any [`ReadBack`]er.
///
/// It's basically the same as `BufReader` just for reading back instead of forward.
///
/// # Examples
/// ```no_run
/// use std::io::{BufReader, Read};
/// use std::fs::File;
/// use read_collection::{BufReadBacker, ReadBack};
///
/// fn main() -> std::io::Result<()> {
///     let file = File::open("some/path")?;
///     let mut reader = BufReader::new(file);
///
///     let mut buffer = Vec::new();
///     reader.read(&mut buffer).unwrap();
///
///
///     // let's read the stuff back in
///     let mut buffer2 = Vec::new();
///     let mut reader = BufReadBacker::from(reader);
///     reader.read_back(&mut buffer2)?;
///
///     assert_eq!(buffer, buffer2);
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct BufReadBacker<R> {
    inner: R,
    buf: Buffer,
}

impl<R> BufReadBacker<R> {
    /// Returns a reference to the internally buffered data.
    ///
    /// Unlike [read_back_fill_buf], this will not attempt to fill the buffer if it is empty.
    ///
    /// # Example
    /// ```no_run
    /// use read_collection::{BufReadBacker, BufReadBack};
    /// use std::fs::File;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let f = File::open("log.txt")?;
    ///     let mut reader = BufReadBacker::new(f);
    ///     assert!(reader.buffer().is_empty());
    ///
    ///     if reader.read_back_fill_buf()?.len() > 0 {
    ///         assert!(!reader.buffer().is_empty());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [read_back_fill_buf]: BufReadBack::read_back_fill_buf
    pub fn buffer(&self) -> &[u8] {
        self.buf.buffer()
    }

    /// Gets a reference to the underlying reader.
    ///
    /// It is inadvisable to directly read from the underlying reader.
    ///
    /// # Example
    /// ```no_run
    /// use read_collection::BufReadBacker;
    /// use std::fs::File;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let f1 = File::open("log.txt")?;
    ///     let reader = BufReadBacker::new(f1);
    ///
    ///     let f2 = reader.get_ref();
    ///     Ok(())
    /// }
    /// ```
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Gets a mutable reference to the underlying reader.
    ///
    /// It is inadvisable to directly read from the underlying reader.
    ///
    /// # Example
    /// ```no_run
    /// use read_collection::BufReadBacker;
    /// use std::fs::File;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let f1 = File::open("log.txt")?;
    ///     let mut reader = BufReadBacker::new(f1);
    ///
    ///     let f2 = reader.get_mut();
    ///     Ok(())
    /// }
    /// ```
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns the number of bytes the internal buffer can hold at once.
    ///
    /// # Example
    /// ```
    /// use read_collection::BufReadBacker;
    ///
    /// fn main() {
    ///     let data: [u8; 5] = [1, 2, 3, 4, 5];
    ///     let reader = BufReadBacker::with_capacity(42, data.as_slice());
    ///
    ///     assert_eq!(reader.capacity(), 42);
    /// }
    /// ````
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    /// Unwraps this `BufReadBacker<R>`, returning the underlying reader.
    ///
    /// Note that any leftover data in the internal buffer is lost. Therefore, a following read from the underlying reader may lead to data loss.
    ///
    /// # Example
    /// ```
    /// use read_collection::{BufReadBacker, ReadBack};
    ///
    /// fn main() {
    ///     let data: [u8; 5] = [1, 2, 3, 4, 5];
    ///     let mut buffer: [u8; 5] = [0; 5];
    ///
    ///     let mut reader = BufReadBacker::new(data.as_slice());
    ///     assert_eq!(reader.read_back(&mut buffer).ok(), Some(5));
    ///
    ///     let mut inner = reader.into_inner();
    ///     // note how the inner reference is "empty" now.
    ///     assert_eq!(inner.read_back(&mut buffer).ok(), Some(0));
    /// }
    /// ```
    pub fn into_inner(self) -> R {
        self.inner
    }

    pub(crate) fn discard_buffer(&mut self) {
        self.buf.discard_buffer();
    }
}

impl<R: ReadBack> BufReadBacker<R> {
    /// Creates a new `BufReadBacker<R>` with a default buffer capacity. The default is currently 8 KiB (or 512 B for bare metal platforms), but may change
    /// in the future.
    ///
    /// # Example
    /// ```no_run
    /// use read_collection::BufReadBacker;
    /// use std::fs::File;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let file = File::open("amogus.txt")?;
    ///     let reader = BufReadBacker::new(file);
    ///     Ok(())
    /// }
    /// ```
    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    /// Creates a new `BufReadBacker<R>` with the specified buffer capacity.
    ///
    /// # Examples
    /// ```no_run
    /// use read_collection::BufReadBacker;
    /// use std::fs::File;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     let nice = File::open("amogus.txt")?;
    ///     let reader = BufReadBacker::with_capacity(69, nice);
    ///     Ok(())
    /// }
    /// ```
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

impl<R: ReadBack> From<BufReader<R>> for BufReadBacker<R> {
    fn from(value: BufReader<R>) -> Self {
        Self::new(value.into_inner())
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
