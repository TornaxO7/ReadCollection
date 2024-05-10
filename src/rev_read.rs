use std::{
    cmp,
    io::{self, ErrorKind, IoSliceMut, Result},
    slice,
};

use crate::{rev_read_borrowed_buf::RevBorrowedCursor, RevBorrowedBuf};

/// Equals the [std::io::Read] trait, except that everything is in reverse.
pub trait RevRead {
    fn rev_read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn rev_read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        default_rev_read_vectored(|b| self.rev_read(b), bufs)
    }
    fn rev_is_read_vectored(&self) -> bool {
        false
    }
    fn rev_read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        default_rev_read_to_end(self, buf, None)
    }
    fn rev_read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        default_rev_read_to_string(self, buf, None)
    }
    fn rev_read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        default_rev_read_exact(self, buf)
    }
    fn rev_read_buf(&mut self, cursor: RevBorrowedCursor<'_>) -> Result<()> {
        default_rev_read_buf(|b| self.rev_read(b), cursor)
    }
    fn rev_read_buf_exact(&mut self, cursor: RevBorrowedCursor<'_>) -> Result<()> {
        default_rev_read_buf_exact(self, cursor)
    }
    fn rev_by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
    fn rev_bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        Bytes { inner: self }
    }
    fn rev_chain<R: io::Read>(self, next: R) -> Chain<Self, R>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: next,
            done_first: false,
        }
    }
    fn rev_take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        Take { inner: self, limit }
    }
}

/// Equals the [std::io::BufRead] trait, except that everything is in reverse.
pub trait RevBufRead: RevRead {
    fn rev_fill_buf(&mut self) -> io::Result<&[u8]>;

    fn rev_consume(&mut self, amt: usize);

    // Provided methods
    fn rev_has_data_left(&mut self) -> io::Result<bool> {
        todo!()
    }

    fn rev_read_until(&mut self, _byte: u8, _buf: &mut Vec<u8>) -> io::Result<usize> {
        todo!()
    }

    fn rev_skip_until(&mut self, _byte: u8) -> io::Result<usize> {
        todo!()
    }

    fn rev_read_line(&mut self, _buf: &mut String) -> io::Result<usize> {
        todo!()
    }

    fn rev_split(self, _byte: u8) -> Split<Self>
    where
        Self: Sized,
    {
        todo!()
    }

    fn rev_lines(self) -> Lines<Self>
    where
        Self: Sized,
    {
        todo!()
    }
}

/// An iterator over `u8` values of a reader.
///
/// This struct is generally created by calling [`bytes`] on a reader.
/// Please see the documentation of [`bytes`] for more details.
///
/// [`bytes`]: Read::bytes
#[derive(Debug)]
pub struct Bytes<R> {
    pub inner: R,
}

impl<R: RevRead> Iterator for Bytes<R> {
    type Item = Result<u8>;

    // Not `#[inline]`. This function gets inlined even without it, but having
    // the inline annotation can result in worse code generation. See #116785.
    fn next(&mut self) -> Option<Result<u8>> {
        SpecReadByte::spec_read_byte(&mut self.inner)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        SizeHint::size_hint(&self.inner)
    }
}

/// For the specialization of `Bytes::next`.
trait SpecReadByte {
    fn spec_read_byte(&mut self) -> Option<Result<u8>>;
}

impl<R> SpecReadByte for R
where
    Self: RevRead,
{
    #[inline]
    fn spec_read_byte(&mut self) -> Option<Result<u8>> {
        inlined_slow_read_byte(self)
    }
}

trait SizeHint {
    fn lower_bound(&self) -> usize;

    fn upper_bound(&self) -> Option<usize>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.lower_bound(), self.upper_bound())
    }
}

impl<T: ?Sized> SizeHint for T {
    #[inline]
    fn lower_bound(&self) -> usize {
        0
    }

    #[inline]
    fn upper_bound(&self) -> Option<usize> {
        None
    }
}

/// Read a single byte in a slow, generic way. This is used by the default
/// `spec_read_byte`.
#[inline]
fn inlined_slow_read_byte<R: RevRead>(reader: &mut R) -> Option<Result<u8>> {
    let mut byte = 0;
    loop {
        return match reader.rev_read(slice::from_mut(&mut byte)) {
            Ok(0) => None,
            Ok(..) => Some(Ok(byte)),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => Some(Err(e)),
        };
    }
}

/// Adapter to chain together two readers.
///
/// This struct is generally created by calling [`chain`] on a reader.
/// Please see the documentation of [`chain`] for more details.
///
/// [`chain`]: Read::chain
#[derive(Debug)]
pub struct Chain<T, U> {
    first: T,
    second: U,
    done_first: bool,
}

impl<T, U> Chain<T, U> {
    /// Consumes the `Chain`, returning the wrapped readers.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io;
    /// use std::io::prelude::*;
    /// use std::fs::File;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut foo_file = File::open("foo.txt")?;
    ///     let mut bar_file = File::open("bar.txt")?;
    ///
    ///     let chain = foo_file.chain(bar_file);
    ///     let (foo_file, bar_file) = chain.into_inner();
    ///     Ok(())
    /// }
    /// ```
    pub fn into_inner(self) -> (T, U) {
        (self.first, self.second)
    }

    /// Gets references to the underlying readers in this `Chain`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io;
    /// use std::io::prelude::*;
    /// use std::fs::File;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut foo_file = File::open("foo.txt")?;
    ///     let mut bar_file = File::open("bar.txt")?;
    ///
    ///     let chain = foo_file.chain(bar_file);
    ///     let (foo_file, bar_file) = chain.get_ref();
    ///     Ok(())
    /// }
    /// ```
    pub fn get_ref(&self) -> (&T, &U) {
        (&self.first, &self.second)
    }

    /// Gets mutable references to the underlying readers in this `Chain`.
    ///
    /// Care should be taken to avoid modifying the internal I/O state of the
    /// underlying readers as doing so may corrupt the internal state of this
    /// `Chain`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::io;
    /// use std::io::prelude::*;
    /// use std::fs::File;
    ///
    /// fn main() -> io::Result<()> {
    ///     let mut foo_file = File::open("foo.txt")?;
    ///     let mut bar_file = File::open("bar.txt")?;
    ///
    ///     let mut chain = foo_file.chain(bar_file);
    ///     let (foo_file, bar_file) = chain.get_mut();
    ///     Ok(())
    /// }
    /// ```
    pub fn get_mut(&mut self) -> (&mut T, &mut U) {
        (&mut self.first, &mut self.second)
    }
}

impl<T: RevRead, U: RevRead> RevRead for Chain<T, U> {
    fn rev_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if !self.done_first {
            match self.first.rev_read(buf)? {
                0 if !buf.is_empty() => self.done_first = true,
                n => return Ok(n),
            }
        }
        self.second.rev_read(buf)
    }

    fn rev_read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        if !self.done_first {
            match self.first.rev_read_vectored(bufs)? {
                0 if bufs.iter().any(|b| !b.is_empty()) => self.done_first = true,
                n => return Ok(n),
            }
        }
        self.second.rev_read_vectored(bufs)
    }

    #[inline]
    fn rev_is_read_vectored(&self) -> bool {
        self.first.rev_is_read_vectored() || self.second.rev_is_read_vectored()
    }

    fn rev_read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let mut read = 0;
        if !self.done_first {
            read += self.first.rev_read_to_end(buf)?;
            self.done_first = true;
        }
        read += self.second.rev_read_to_end(buf)?;
        Ok(read)
    }

    // We don't override `read_to_string` here because an UTF-8 sequence could
    // be split between the two parts of the chain
    fn rev_read_buf(&mut self, mut buf: RevBorrowedCursor<'_>) -> Result<()> {
        if buf.capacity() == 0 {
            return Ok(());
        }

        if !self.done_first {
            let old_len = buf.written();
            self.first.rev_read_buf(buf.reborrow())?;

            if buf.written() != old_len {
                return Ok(());
            } else {
                self.done_first = true;
            }
        }
        self.second.rev_read_buf(buf)
    }
}

impl<T: RevBufRead, U: RevBufRead> RevBufRead for Chain<T, U> {
    fn rev_fill_buf(&mut self) -> Result<&[u8]> {
        if !self.done_first {
            match self.first.rev_fill_buf()? {
                [] => self.done_first = true,
                buf => return Ok(buf),
            }
        }
        self.second.rev_fill_buf()
    }

    fn rev_consume(&mut self, amt: usize) {
        if !self.done_first {
            self.first.rev_consume(amt)
        } else {
            self.second.rev_consume(amt)
        }
    }

    fn rev_read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> Result<usize> {
        let mut read = 0;
        if !self.done_first {
            let n = self.first.rev_read_until(byte, buf)?;
            read += n;

            match buf.last() {
                Some(b) if *b == byte && n != 0 => return Ok(read),
                _ => self.done_first = true,
            }
        }
        read += self.second.rev_read_until(byte, buf)?;
        Ok(read)
    }
}

/// An iterator over the contents of an instance of `BufRead` split on a
/// particular byte.
///
/// This struct is generally created by calling [`split`] on a `BufRead`.
/// Please see the documentation of [`split`] for more details.
///
/// [`split`]: BufRead::split
#[derive(Debug)]
pub struct Split<B> {
    buf: B,
    delim: u8,
}

impl<B: RevBufRead> Iterator for Split<B> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.rev_read_until(self.delim, &mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf[buf.len() - 1] == self.delim {
                    buf.pop();
                }
                Some(Ok(buf))
            }
            Err(e) => Some(Err(e)),
        }
    }
}

/// An iterator over the lines of an instance of `BufRead`.
///
/// This struct is generally created by calling [`lines`] on a `BufRead`.
/// Please see the documentation of [`lines`] for more details.
///
/// [`lines`]: BufRead::lines
#[derive(Debug)]
pub struct Lines<B> {
    buf: B,
}

impl<B: RevBufRead> Iterator for Lines<B> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Result<String>> {
        let mut buf = String::new();
        match self.buf.rev_read_line(&mut buf) {
            Ok(0) => None,
            Ok(_n) => {
                if buf.ends_with('\n') {
                    buf.pop();
                    if buf.ends_with('\r') {
                        buf.pop();
                    }
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
pub struct Take<T> {
    inner: T,
    limit: u64,
}

impl<T> Take<T> {
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

impl<T: RevRead> RevRead for Take<T> {
    fn rev_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(0);
        }

        let max = cmp::min(buf.len() as u64, self.limit) as usize;
        let n = self.inner.rev_read(&mut buf[..max])?;
        assert!(n as u64 <= self.limit, "number of read bytes exceeds limit");
        self.limit -= n as u64;
        Ok(n)
    }

    fn rev_read_buf(&mut self, mut buf: RevBorrowedCursor<'_>) -> Result<()> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(());
        }

        if self.limit <= buf.capacity() as u64 {
            // if we just use an as cast to convert, limit may wrap around on a 32 bit target
            let limit = cmp::min(self.limit, usize::MAX as u64) as usize;

            let extra_init = cmp::min(limit as usize, buf.init_ref().len());

            // SAFETY: no uninit data is written to ibuf
            let ibuf = unsafe { &mut buf.as_mut()[..limit] };

            let mut sliced_buf: RevBorrowedBuf<'_> = ibuf.into();

            // SAFETY: extra_init bytes of ibuf are known to be initialized
            unsafe {
                sliced_buf.set_init(extra_init);
            }

            let mut cursor = sliced_buf.unfilled();
            self.inner.rev_read_buf(cursor.reborrow())?;

            let new_init = cursor.init_ref().len();
            let filled = sliced_buf.len();

            // cursor / sliced_buf / ibuf must drop here

            unsafe {
                // SAFETY: filled bytes have been filled and therefore initialized
                buf.advance(filled);
                // SAFETY: new_init bytes of buf's unfilled buffer have been initialized
                buf.set_init(new_init);
            }

            self.limit -= filled as u64;
        } else {
            let written = buf.written();
            self.inner.rev_read_buf(buf.reborrow())?;
            self.limit -= (buf.written() - written) as u64;
        }

        Ok(())
    }
}

impl<T: RevBufRead> RevBufRead for Take<T> {
    fn rev_fill_buf(&mut self) -> Result<&[u8]> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(&[]);
        }

        let buf = self.inner.rev_fill_buf()?;
        let cap = cmp::min(buf.len() as u64, self.limit) as usize;
        Ok(&buf[..cap])
    }

    fn rev_consume(&mut self, amt: usize) {
        // Don't let callers reset the limit by passing an overlarge value
        let amt = cmp::min(amt as u64, self.limit) as usize;
        self.limit -= amt as u64;
        self.inner.rev_consume(amt);
    }
}

/// == default implementations ==
pub fn default_rev_read_vectored<F>(rev_read: F, bufs: &mut [IoSliceMut<'_>]) -> Result<usize>
where
    F: FnOnce(&mut [u8]) -> Result<usize>,
{
    let buf = bufs
        .iter_mut()
        .find(|b| !b.is_empty())
        .map_or(&mut [][..], |b| &mut **b);
    rev_read(buf)
}

pub fn default_rev_read_to_end<R: RevRead + ?Sized>(
    _r: &mut R,
    _buf: &mut [u8],
    _size_hint: Option<usize>,
) -> Result<usize> {
    todo!()
}

pub fn default_rev_read_to_string<R: RevRead + ?Sized>(
    _r: &mut R,
    _buf: &mut str,
    _size_hint: Option<usize>,
) -> Result<usize> {
    todo!()
}

pub fn default_rev_read_exact<R: RevRead + ?Sized>(this: &mut R, mut buf: &mut [u8]) -> Result<()> {
    while !buf.is_empty() {
        match this.rev_read(buf) {
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
            "failed to fill whole buffer",
        ))
    } else {
        Ok(())
    }
}

pub fn default_rev_read_buf<F>(read: F, mut cursor: RevBorrowedCursor<'_>) -> Result<()>
where
    F: FnOnce(&mut [u8]) -> Result<usize>,
{
    let n = read(cursor.ensure_init().init_mut())?;
    unsafe {
        // SAFETY: we initialised using `ensure_init` so there is no uninit data to advance to.
        cursor.advance(n);
    }
    Ok(())
}

pub fn default_rev_read_buf_exact<R: RevRead + ?Sized>(
    read: &mut R,
    mut cursor: RevBorrowedCursor<'_>,
) -> Result<()> {
    while cursor.capacity() > 0 {
        let prev_written = cursor.written();
        match read.rev_read_buf(cursor.reborrow()) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }

        if cursor.written() == prev_written {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "failed to fill buffer",
            ));
        }
    }

    Ok(())
}
