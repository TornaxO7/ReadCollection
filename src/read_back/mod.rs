mod buf_read_backer;
mod impls;

pub use buf_read_backer::BufReadBacker;

use std::{
    cmp,
    io::{self, ErrorKind, IoSliceMut, Result},
    slice,
};

use crate::DEFAULT_BUF_SIZE;

/// A trait to read back the content which has been read with the methods of [std::io::Read].
///
/// # Example
/// ```no_run
/// use read_collection::ReadBack;
/// use std::io::Read;
/// use std::fs::File;
///
/// fn main() {
///     let mut file = File::open("some/path").unwrap();
///     let mut read_buffer: [u8; 10] = [0; 10];
///     let mut read_back_buffer: [u8; 10] = [0; 10];
///
///     // well, let's read some stuff!
///     file.read(&mut read_buffer).unwrap();
///
///     // do some work and somehow clear the `read_buffer`
///     // ....
///
///     // why not read it again?
///     file.read_back(&mut read_back_buffer).unwrap();
///
///     assert_eq!(read_buffer, read_back_buffer);
/// }
/// ```
pub trait ReadBack {
    /// Pull some bytes from this source into the specified buffer, returning how many bytes were read.
    ///
    /// The same conditions have to be met as in [`Read::read`].
    /// The difference to [`Read::read`] is that `read_back` is reading "backwards".
    ///
    /// # Example
    /// ```rust
    /// use read_collection::ReadBack;
    ///
    /// fn main() {
    ///     let data = [1u8, 2u8];
    ///     let mut buffer: [u8; 3] = [0; 3];
    ///     let mut small_buffer: [u8; 1] = [0];
    ///
    ///     assert_eq!(data.as_slice().read_back(&mut buffer).ok(), Some(2));
    ///     assert_eq!(data.as_slice().read_back(&mut small_buffer).ok(), Some(1));
    ///
    ///     // notice here, that the values are added at the beginning of the array!
    ///     assert_eq!(&buffer, &[1, 2, 0]);
    ///
    ///     // notice here that the last value of data gets inserted "first"
    ///     assert_eq!(&small_buffer, &[2]);
    /// }
    /// ```
    ///
    /// [`Read::read`]: std::io::Read::read
    fn read_back(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Like [`Read::read_vectored`] but it uses `rev_read` instead of `read`.
    ///
    /// [`Read::read_vectored`]: std::io::Read::read_vectored
    fn read_back_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        default_read_back_vectored(|b| self.read_back(b), bufs)
    }

    /// Read all bytes until the start of the source, placing them into `buf`.
    ///
    /// Can be also seen as "read back until you reach the start of the source".
    ///
    /// # Example
    /// ```no_run
    /// use std::fs::File;
    /// use std::io::Read;
    /// use read_collection::ReadBack;
    ///
    /// fn main() {
    ///     let mut file = File::open("some/path").unwrap();
    ///     let mut buffer: Vec<u8> = vec![0; 100];
    ///
    ///     // do some arbitrary read...
    ///     file.read(&mut buffer).unwrap();
    ///
    ///     // do some even *more* random read...
    ///     // ...
    ///
    ///     // well, why not collecting *everything* from the start to
    ///     // the cursor position?
    ///     file.read_back_to_end(&mut buffer).unwrap();
    /// }
    /// ```
    fn read_back_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        default_read_back_to_end(self, buf)
    }

    /// Read all bytes until the start of the source, **pre**pending them to `buf` (since we're reading back).
    ///
    /// # Example
    /// ```
    /// use read_collection::ReadBack;
    ///
    /// fn main() {
    ///     let mut message = "Arch btw.".to_string();
    ///     let prefix = b"I use ";
    ///
    ///     assert_eq!(prefix.as_slice().read_back_to_string(&mut message).ok(), Some(prefix.len()));
    ///     assert_eq!(message, "I use Arch btw.".to_string());
    /// }
    /// ```
    fn read_back_to_string(&mut self, buf: &mut String) -> Result<usize> {
        default_read_back_to_string(self, buf)
    }

    /// Read back the exact number of bytes required to fill `buf`.
    ///
    /// The conditions for [`Read::read_exact`] apply here as well.
    ///
    /// # Example
    /// ```
    /// use read_collection::ReadBack;
    ///
    /// fn main() {
    ///     let values = [1, 2, 3];
    ///     let mut buffer = [0, 0];
    ///
    ///     assert!(values.as_slice().read_back_exact(&mut buffer).is_ok());
    ///     assert_eq!(buffer, [2, 3]);
    /// }
    /// ```
    ///
    /// [`Read::read_exact`]: std::io::Read::read_exact
    fn read_back_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        default_read_back_exact(self, buf)
    }

    /// Transforms this `ReadBack` instance to an `Iterator` over its bytes.
    /// This can be also seen as "read the bytes of the instance in reverse".
    ///
    /// The same conditions of [`Read::bytes`] apply here as well.
    ///
    /// # Example
    /// ```
    /// use read_collection::ReadBack;
    ///
    /// fn main() {
    ///     let data = [1, 2, 3];
    ///
    ///     let mut iterator = data.as_slice().read_back_bytes();
    ///     let read_back_bytes = iterator
    ///         .map(|b| b.unwrap())
    ///         .collect::<Vec<u8>>();
    ///
    ///     assert_eq!(read_back_bytes, [3, 2, 1].to_vec());
    /// }
    /// ```
    ///
    /// [`Read::bytes`]: std::io::Read::bytes
    fn read_back_bytes(self) -> ReadBackBytes<Self>
    where
        Self: Sized,
    {
        ReadBackBytes { inner: self }
    }

    /// Creates an adapter which will chain this stream with another.
    ///
    /// # Example
    /// ```
    /// use read_collection::ReadBack;
    ///
    /// fn main() {
    ///     let first_data = b"First in the chain.";
    ///     let second_data = b" Second in the chain.";
    ///     let total_length = first_data.len() + second_data.len();
    ///     let mut buffer: Vec<u8> = vec![0; total_length];
    ///
    ///     let mut chain = first_data.read_back_chain(second_data.as_slice());
    ///
    ///     assert_eq!(chain.read_back(&mut buffer).ok(), Some(first_data.len()));
    ///     assert_eq!(chain.read_back(&mut buffer[first_data.len()..]).ok(), Some(second_data.len()));
    ///
    ///     assert_eq!(String::from_utf8(buffer).unwrap(), "First in the chain. Second in the chain.".to_string());
    /// }
    /// ```
    fn read_back_chain<R: ReadBack>(self, next: R) -> ReadBackChain<Self, R>
    where
        Self: Sized,
    {
        ReadBackChain {
            first: self,
            second: next,
            done_first: false,
        }
    }

    /// Creates an adapter which will read at most `limit` bytes from it.
    ///
    /// # Example
    ///
    /// ```
    /// use read_collection::ReadBack;
    ///
    /// fn main() {
    ///     let data: [u8; 3] = [1, 2, 3];
    ///     let mut buffer: [u8; 3] = [0; 3];
    ///
    ///     let mut take = data.as_slice().read_back_take(2);
    ///
    ///     assert_eq!(take.read_back(&mut buffer).ok(), Some(2));
    ///     // we've already read 2, so we reached the limit
    ///     assert_eq!(take.read_back(&mut buffer).ok(), Some(0));
    ///
    ///     assert_eq!(buffer, [2, 3, 0]);
    /// }
    /// ```
    fn read_back_take(self, limit: u64) -> ReadBackTake<Self>
    where
        Self: Sized,
    {
        ReadBackTake { inner: self, limit }
    }
}

/// A `BufReadBack` is a type of [`ReadBack`]er which has an internal buffer, allowing it to perform extra ways of reading.
///
/// It behaves the same as [`BufRead`] except that it uses [`ReadBack`] internally.
///
/// [`ReadBack`]: ReadBack
/// [`BufRead`]: std::io::BufRead
pub trait BufReadBack: ReadBack {
    /// Returns the contents of the internal buffer, filling it with more data from the inner reader if it is empty.
    ///
    /// This function is a lower-level call.
    /// It needs to be paired with the [`read_back_consume`] method to function properly.
    /// When calling this method, none of the contents will be "read back" in the sense that later calling [`read_back`] may return the same contents.
    /// As such, consume must be called with the number of bytes that are consumed from this buffer to ensure that the bytes are never returned twice.
    ///
    /// An empty buffer returned indicates that the stream has reached the beginning again.
    ///
    /// # Error
    /// This function will return an I/O error if the underlying reader was read, but returned an error.
    ///
    /// # Example
    /// TODO
    ///
    /// [`read_back_consume`]: BufReadBack::read_back_consume
    /// [`read_back`]: ReadBack::read_back
    fn read_back_fill_buf(&mut self) -> io::Result<&[u8]>;

    /// Tells this buffer that `amt` bytes have been consumed from the buffer, so they should no longer be returned in calls to [`read_back`].
    ///
    /// It basically behaves the same as [`BufRead::consume`] except that you should combine this with [`read_back_fill_buf`].
    ///
    /// [`read_back_fill_buf`]: BufReadBack::read_back_fill_buf
    /// [`BufRead::consume`]: std::io::BufRead::consume
    /// [`read_back`]: ReadBack::read_back
    fn read_back_consume(&mut self, amt: usize);

    /// Check if the underlying [`ReadBack`] has any data left to be read.
    ///
    /// This function may fill the buffer to check for data,
    /// so this functions returns `Result<bool>`, not `bool`.
    ///
    /// Default implementation calls [`read_back_fill_buf`] and checks that
    /// returned slice is empty (which means that there is no data left,
    /// since the start is reached).
    ///
    /// # Example
    /// TODO
    ///
    /// [`ReadBack`]: ReadBack
    /// [`read_back_fill_buf`]: BufReadBack::read_back_fill_buf
    fn read_back_has_data_left(&mut self) -> io::Result<bool> {
        self.read_back_fill_buf().map(|buffer| buffer.is_empty())
    }

    /// Read all bytes into `buf` until the delimiter `byte` or the beginning of the reader is reached.
    ///
    /// This function will read bytes from the underlying stream until the delimiter or the beginning of the reader is reached.
    /// Once found, all bytes up to, and including, the delimiter (if found) will be appended to buf.
    ///
    /// If successful, this function will return the total number of bytes read.
    ///
    /// # Example
    /// TODO
    fn read_back_until(&mut self, delim: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        default_buf_read_back_until(self, delim, buf)
    }

    /// Skip all bytes until the delimiter byte or the beginning is reached.
    ///
    /// This function will read (and discard) bytes from the underlying stream until the delimiter or EOF is found.
    ///
    /// If successful, this function will return the total number of bytes read, including the delimiter byte.
    ///
    /// This is useful for efficiently skipping data such as NUL-terminated strings in binary file formats without buffering.
    ///
    /// # Example
    fn read_back_skip_until(&mut self, delim: u8) -> io::Result<usize> {
        default_buf_read_skip_until(self, delim)
    }

    /// Read all bytes until a newline (the `0xA` byte) is reached, and *prepend* them to the provided String buffer.
    ///
    /// This function also behaves similar as [`BufRead::read_line`] except that it uses the functions of [`ReadBack`] instead
    /// of [`Read`].
    ///
    /// # Example
    /// TODO
    ///
    /// [`BufRead::read_line`]: std::io::BufRead::read_line
    /// [`clear`]: std::string::String::clear
    /// [`ReadBack`]: ReadBack
    /// [`Read`]: std::io::Read
    fn read_back_line(&mut self, dest: &mut String) -> io::Result<usize> {
        default_buf_read_back_line(self, dest)
    }

    /// Returns an iterator over the contents of this reader split on the byte byte.
    ///
    /// This function also behaves similar as [`BufRead::split`] except that it uses the functions of [`ReadBack`] instead
    /// of [`Read`].
    ///
    /// # Example
    /// TODO
    ///
    /// [`BufRead::split`]: std::io::BufRead::split
    /// [`ReadBack`]: ReadBack
    /// [`Read`]: std::io::Read
    fn read_back_split(self, delim: u8) -> ReadBackSplit<Self>
    where
        Self: Sized,
    {
        ReadBackSplit { buf: self, delim }
    }

    /// Returns an iterator over the lines of this reader.
    ///
    /// This function also behaves similar as [`BufRead::lines`] except that it uses the functions of [`ReadBack`] instead
    /// of [`Read`].
    ///
    /// # Example
    /// TODO
    ///
    /// [`BufRead::lines`]: std::io::BufRead::lines
    /// [`ReadBack`]: ReadBack
    /// [`Read`]: std::io::Read
    fn read_back_lines(self) -> RevLines<Self>
    where
        Self: Sized,
    {
        RevLines { buf: self }
    }
}

/// An iterator over `u8` values of a read-back reader.
///
/// This struct is generally created by calling [`read_back_bytes`] on a [`ReadBack`] reader.
/// Please see the documentation of [`read_back_bytes`] for more details.
///
/// [`read_back_bytes`]: ReadBack::read_back_bytes
/// [`ReadBack`]: ReadBack
#[derive(Debug)]
pub struct ReadBackBytes<R> {
    inner: R,
}

impl<R: ReadBack> Iterator for ReadBackBytes<R> {
    type Item = Result<u8>;

    // Not `#[inline]`. This function gets inlined even without it, but having
    // the inline annotation can result in worse code generation. See #116785.
    fn next(&mut self) -> Option<Result<u8>> {
        let mut byte: u8 = 0;
        loop {
            return match self.inner.read_back(slice::from_mut(&mut byte)) {
                Ok(0) => None,
                Err(e) if e.kind() == ErrorKind::Other => None,
                Ok(..) => Some(Ok(byte)),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => Some(Err(e)),
            };
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// Adapter to chain together two [`ReadBack`]s.
///
/// This struct is generally created by calling [`read_back_chain`] on a reader.
/// Please see the documentation of [`read_back_chain`] for more details.
///
/// [`ReadBack`]: ReadBack
/// [`read_back_chain`]: ReadBack::read_back_chain
#[derive(Debug)]
pub struct ReadBackChain<T, U> {
    first: T,
    second: U,
    done_first: bool,
}

impl<T, U> ReadBackChain<T, U> {
    /// Consumes the [`ReadBackChain`], returning the wrapped [readers].
    ///
    /// # Examples
    /// ```
    /// use std::io;
    /// use read_collection::ReadBack;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut data1: [u8; 3] = [1, 2, 3];
    ///     let mut data2: [u8; 3] = [4, 5, 6];
    ///
    ///     let read_back_chain = data1.as_slice().read_back_chain(data2.as_slice());
    ///     let (data1, data2) = read_back_chain.into_inner();
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`ReadBackChain`]: ReadBackChain
    /// [readers]: ReadBack
    pub fn into_inner(self) -> (T, U) {
        (self.first, self.second)
    }

    /// Gets references to the underlying [readers] in this [`ReadBackChain`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io;
    /// use read_collection::ReadBack;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut data1: [u8; 3] = [1, 2, 3];
    ///     let mut data2: [u8; 3] = [4, 5, 6];
    ///
    ///     let read_back_chain = data1.as_slice().read_back_chain(data2.as_slice());
    ///     let (data1, data2) = read_back_chain.get_ref();
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`ReadBackChain`]: ReadBackChain
    /// [readers]: ReadBack
    pub fn get_ref(&self) -> (&T, &U) {
        (&self.first, &self.second)
    }

    /// Gets mutable references to the underlying [readers] in this [`ReadBackChain`].
    ///
    /// Care should be taken to avoid modifying the internal I/O state of the
    /// underlying [readers] as doing so may corrupt the internal state of this
    /// [`ReadBackChain`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io;
    /// use read_collection::ReadBack;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut data1: [u8; 3] = [1, 2, 3];
    ///     let mut data2: [u8; 3] = [4, 5, 6];
    ///
    ///     let mut read_back_chain = data1.as_slice().read_back_chain(data2.as_slice());
    ///     let (data1, data2) = read_back_chain.get_mut();
    ///     Ok(())
    /// }
    /// ```
    ///
    /// [`ReadBackChain`]: ReadBackChain
    /// [readers]: ReadBack
    pub fn get_mut(&mut self) -> (&mut T, &mut U) {
        (&mut self.first, &mut self.second)
    }
}

impl<T: ReadBack, U: ReadBack> ReadBack for ReadBackChain<T, U> {
    fn read_back(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.done_first {
            match self.first.read_back(buf)? {
                0 if !buf.is_empty() => self.done_first = true,
                n => return Ok(n),
            }
        }
        self.second.read_back(buf)
    }

    fn read_back_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        if !self.done_first {
            match self.first.read_back_vectored(bufs)? {
                0 if bufs.iter().any(|b| !b.is_empty()) => self.done_first = true,
                n => return Ok(n),
            }
        }
        self.second.read_back_vectored(bufs)
    }

    fn read_back_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let mut read = 0;
        if !self.done_first {
            read += self.first.read_back_to_end(buf)?;
            self.done_first = true;
        }
        read += self.second.read_back_to_end(buf)?;
        Ok(read)
    }
}

impl<T: BufReadBack, U: BufReadBack> BufReadBack for ReadBackChain<T, U> {
    fn read_back_fill_buf(&mut self) -> Result<&[u8]> {
        if !self.done_first {
            match self.first.read_back_fill_buf()? {
                [] => self.done_first = true,
                buf => return Ok(buf),
            }
        }
        self.second.read_back_fill_buf()
    }

    fn read_back_consume(&mut self, amt: usize) {
        if !self.done_first {
            self.first.read_back_consume(amt)
        } else {
            self.second.read_back_consume(amt)
        }
    }

    fn read_back_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        let mut read = 0;
        if !self.done_first {
            let n = self.first.read_back_until(byte, buf)?;
            read += n;

            match buf.last() {
                Some(b) if *b == byte && n != 0 => return Ok(read),
                _ => self.done_first = true,
            }
        }
        read += self.second.read_back_until(byte, buf)?;
        Ok(read)
    }
}

/// An iterator over the contents of an instance of [`BufReadBack`] split on a
/// particular byte.
///
/// This struct is generally created by calling [`read_back_split`] on a [`BufReadBack`].
/// Please see the documentation of [`read_back_split`] for more details.
///
/// [`BufReadBack`]: BufReadBack
/// [`read_back_split`]: BufReadBack::read_back_split
#[derive(Debug)]
pub struct ReadBackSplit<B> {
    buf: B,
    delim: u8,
}

impl<B: BufReadBack> Iterator for ReadBackSplit<B> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.read_back_until(self.delim, &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf[0] == self.delim {
                    buf.drain(..1);
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// An iterator over the lines of an instance of `RevBufRead`.
///
/// This struct is generally created by calling [`rev_lines`] on a `RevBufRead`.
/// Please see the documentation of [`rev_lines`] for more details.
///
/// [`rev_lines`]: RevBufRead::rev_lines
#[derive(Debug)]
pub struct RevLines<B> {
    buf: B,
}

impl<B: BufReadBack> Iterator for RevLines<B> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Result<String>> {
        let mut buf = String::new();
        match self.buf.read_back_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf.starts_with('\n') {
                    buf = buf.drain(1..).collect();
                } else if buf.starts_with("\r\n") {
                    buf = buf.drain(2..).collect();
                }

                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// Reader adapter which limits the bytes read from an underlying reader.
///
/// This struct is generally created by calling [`take`] on a reader.
/// Please see the documentation of [`take`] for more details.
///
/// [`take`]: Read::take
#[derive(Debug)]
pub struct ReadBackTake<T> {
    inner: T,
    limit: u64,
}

impl<T> ReadBackTake<T> {
    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: ReadBack> ReadBack for ReadBackTake<T> {
    fn read_back(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(0);
        }

        let max = cmp::min(buf.len() as u64, self.limit) as usize;
        let n = self.inner.read_back(&mut buf[..max])?;
        assert!(n as u64 <= self.limit, "number of read bytes exceeds limit");
        self.limit -= n as u64;
        Ok(n)
    }
}

impl<T: BufReadBack> BufReadBack for ReadBackTake<T> {
    fn read_back_fill_buf(&mut self) -> Result<&[u8]> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(&[]);
        }

        let buf = self.inner.read_back_fill_buf()?;
        let buf_len = buf.len();

        let cap = cmp::min(buf_len as u64, self.limit) as usize;
        Ok(&buf[buf_len - cap..])
    }

    fn read_back_consume(&mut self, amt: usize) {
        // Don't let callers reset the limit by passing an overlarge value
        let amt = cmp::min(amt as u64, self.limit) as usize;
        self.limit -= amt as u64;
        self.inner.read_back_consume(amt);
    }
}

/// == default implementations ==
pub fn default_read_back_vectored<F: FnOnce(&mut [u8]) -> Result<usize>>(
    read_back: F,
    bufs: &mut [IoSliceMut<'_>],
) -> Result<usize> {
    let buf = bufs
        .iter_mut()
        .find(|b| !b.is_empty())
        .map_or(&mut [][..], |b| &mut **b);

    read_back(buf)
}

pub fn default_read_back_to_end<R: ReadBack + ?Sized>(
    reader: &mut R,
    dest_buf: &mut Vec<u8>,
) -> Result<usize> {
    let mut buffers: Vec<Vec<u8>> = vec![];
    let mut curr_buffer: Vec<u8> = vec![0; DEFAULT_BUF_SIZE];

    let mut amount_read: usize = 0;

    loop {
        match reader.read_back(curr_buffer.as_mut_slice()) {
            Ok(amount) => {
                println!("{}", amount);
                if amount == 0 {
                    let mut final_buf = Vec::with_capacity(amount_read + dest_buf.len());

                    for buffer in buffers.into_iter().rev() {
                        final_buf.extend_from_slice(&buffer);
                    }
                    final_buf.extend_from_slice(dest_buf);
                    *dest_buf = final_buf;

                    return Ok(amount_read);
                }
                curr_buffer = {
                    let curr_buffer_len = curr_buffer.len();
                    curr_buffer[curr_buffer_len - amount..].to_vec()
                };
                amount_read += amount;
                buffers.push(curr_buffer);
                curr_buffer = Vec::new();
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

fn default_read_back_to_string<R: ReadBack + ?Sized>(r: &mut R, buf: &mut String) -> Result<usize> {
    let mut bytes_buf = Vec::new();
    let amount_bytes = default_read_back_to_end(r, &mut bytes_buf)?;

    let mut read_back_string = String::from_utf8(bytes_buf).map_err(|e| {
        std::io::Error::new(
            ErrorKind::InvalidData,
            format!("Couldn't convert the rev-reader to a string: {}", e),
        )
    })?;

    read_back_string.push_str(buf);
    *buf = read_back_string;

    Ok(amount_bytes)
}

fn default_read_back_exact<R: ReadBack + ?Sized>(r: &mut R, mut buf: &mut [u8]) -> Result<()> {
    while !buf.is_empty() {
        match r.read_back(buf) {
            Ok(0) => break,
            Ok(n) => {
                let buf_len = buf.len();
                buf = &mut buf[..buf_len - n];
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }

    if !buf.is_empty() {
        Err(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            "Failed to fill whole buffer.",
        ))
    } else {
        Ok(())
    }
}

fn default_buf_read_back_until<R: BufReadBack + ?Sized>(
    r: &mut R,
    delim: u8,
    buf: &mut Vec<u8>,
) -> io::Result<usize> {
    let mut amount_read = 0;

    loop {
        let (done, used) = {
            let new_read = match r.read_back_fill_buf() {
                Ok(n) => n,
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            };
            match memchr::memrchr(delim, new_read) {
                Some(index) => {
                    let used = new_read.len() - index;

                    let mut new_buf = Vec::with_capacity(buf.len() + used);
                    new_buf.extend_from_slice(&new_read[index..]);
                    new_buf.extend_from_slice(buf);
                    *buf = new_buf;

                    (true, used)
                }
                None => {
                    let mut new_buf = Vec::with_capacity(buf.len() + new_read.len());
                    new_buf.extend_from_slice(new_read);
                    new_buf.extend_from_slice(buf);
                    *buf = new_buf;

                    (false, new_read.len())
                }
            }
        };

        r.read_back_consume(used);
        amount_read += used;
        if done || used == 0 {
            return Ok(amount_read);
        }
    }
}

fn default_buf_read_skip_until<R: BufReadBack + ?Sized>(r: &mut R, delim: u8) -> Result<usize> {
    let mut amount_read: usize = 0;

    loop {
        let (done, used) = {
            let new_read = match r.read_back_fill_buf() {
                Ok(n) => n,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };

            match memchr::memrchr(delim, new_read) {
                Some(index) => (true, new_read.len() - index),
                None => (false, new_read.len()),
            }
        };

        r.read_back_consume(used);
        amount_read += used;
        if done || used == 0 {
            return Ok(amount_read);
        }
    }
}

fn default_buf_read_back_line<R: BufReadBack + ?Sized>(
    r: &mut R,
    dest: &mut String,
) -> io::Result<usize> {
    let mut buffer = Vec::with_capacity(crate::DEFAULT_BUF_SIZE);

    let mut amount_read = r.read_back_until(b'\n', &mut buffer)?;
    if r.read_back_fill_buf()?
        .last()
        .map(|&c| c == b'\r')
        .unwrap_or(false)
    {
        let mut new_buf = Vec::with_capacity(buffer.len() + 1);
        new_buf.push(b'\r');
        new_buf.extend_from_slice(&buffer);
        buffer = new_buf;
        amount_read += 1;
        r.read_back_consume(1);
    }

    match String::from_utf8(buffer) {
        Ok(mut line) => {
            line.push_str(dest);
            *dest = line;

            Ok(amount_read)
        }
        Err(err) => Err(io::Error::new(ErrorKind::InvalidData, err)),
    }
}
