use std::{
    cmp,
    io::{self, ErrorKind, IoSliceMut, Result},
    slice,
};

use crate::{rev_read_borrowed_buf::RevBorrowedCursor, RevBorrowedBuf, DEFAULT_BUF_SIZE};

/// Equals the [std::io::Read] trait, except that everything is in reverse.
pub trait RevRead {
    fn rev_read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn rev_read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        let buf = bufs
            .iter_mut()
            .find(|b| !b.is_empty())
            .map_or(&mut [][..], |b| &mut **b);

        self.rev_read(buf)
    }
    fn rev_is_read_vectored(&self) -> bool {
        false
    }

    fn rev_read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        default_rev_read_to_end(self, buf)
    }

    fn rev_read_to_string(&mut self, buf: &mut String) -> Result<usize> {
        let mut bytes_buf = Vec::new();
        let amount_bytes = default_rev_read_to_end(self, &mut bytes_buf)?;

        let mut rev_read_string = String::from_utf8(bytes_buf).map_err(|e| {
            std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Couldn't convert the rev-reader to a string: {}", e),
            )
        })?;

        rev_read_string.push_str(&buf);
        *buf = rev_read_string;

        Ok(amount_bytes)
    }

    fn rev_read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.rev_read(buf) {
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

    fn rev_bytes(self) -> RevBytes<Self>
    where
        Self: Sized,
    {
        RevBytes { inner: self }
    }

    fn rev_chain<R: RevRead>(self, next: R) -> RevChain<Self, R>
    where
        Self: Sized,
    {
        RevChain {
            first: self,
            second: next,
            done_first: false,
        }
    }
    fn rev_take(self, limit: u64) -> RevTake<Self>
    where
        Self: Sized,
    {
        RevTake { inner: self, limit }
    }
}

/// Equals the [std::io::BufRead] trait, except that everything is in reverse.
pub trait RevBufRead: RevRead {
    fn rev_fill_buf(&mut self) -> io::Result<&[u8]>;

    fn rev_consume(&mut self, amt: usize);

    /// Check if the underlying `RevRead` has any data left to be read.
    ///
    /// This function may fill the buffer to check for data,
    /// so this functions returns `Result<bool>`, not `bool`.
    ///
    /// Default implementation calls `rev_fill_buf` and checks that
    /// returned slice is empty (which means that there is no data left,
    /// since EOF is reached).
    fn rev_has_data_left(&mut self) -> io::Result<bool> {
        self.rev_fill_buf().map(|buffer| buffer.is_empty())
    }

    fn rev_read_until(&mut self, delim: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        let mut amount_read = 0;

        loop {
            let (done, used) = {
                let new_read = match self.rev_fill_buf() {
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

            self.rev_consume(used);
            amount_read += used;
            if done || used == 0 {
                return Ok(amount_read);
            }
        }
    }

    fn rev_skip_until(&mut self, delim: u8) -> io::Result<usize> {
        let mut amount_read: usize = 0;

        loop {
            let (done, used) = {
                let new_read = match self.rev_fill_buf() {
                    Ok(n) => n,
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                };

                match memchr::memrchr(delim, new_read) {
                    Some(index) => (true, new_read.len() - index),
                    None => (false, new_read.len()),
                }
            };

            self.rev_consume(used);
            amount_read += used;
            if done || used == 0 {
                return Ok(amount_read);
            }
        }
    }

    fn rev_read_line(&mut self, dest: &mut String) -> io::Result<usize> {
        let mut buffer = Vec::with_capacity(crate::DEFAULT_BUF_SIZE);

        let mut amount_read = self.rev_read_until(b'\n', &mut buffer)?;
        if self
            .rev_fill_buf()?
            .last()
            .map(|&c| c == b'\r')
            .unwrap_or(false)
        {
            let mut new_buf = Vec::with_capacity(buffer.len() + 1);
            new_buf.push(b'\r');
            new_buf.extend_from_slice(&buffer);
            buffer = new_buf;
            amount_read += 1;
            self.rev_consume(1);
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

    fn rev_split(self, delim: u8) -> RevSplit<Self>
    where
        Self: Sized,
    {
        RevSplit { buf: self, delim }
    }

    fn rev_lines(self) -> RevLines<Self>
    where
        Self: Sized,
    {
        RevLines { buf: self }
    }
}

/// An iterator over `u8` values of a reader.
///
/// This struct is generally created by calling [`bytes`] on a reader.
/// Please see the documentation of [`bytes`] for more details.
///
/// [`bytes`]: Read::bytes
#[derive(Debug)]
pub struct RevBytes<R> {
    pub inner: R,
}

impl<R: RevRead> Iterator for RevBytes<R> {
    type Item = Result<u8>;

    // Not `#[inline]`. This function gets inlined even without it, but having
    // the inline annotation can result in worse code generation. See #116785.
    fn next(&mut self) -> Option<Result<u8>> {
        let mut byte: u8 = 0;
        loop {
            return match self.inner.rev_read(slice::from_mut(&mut byte)) {
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

/// Adapter to chain together two readers.
///
/// This struct is generally created by calling [`chain`] on a reader.
/// Please see the documentation of [`chain`] for more details.
///
/// [`chain`]: Read::chain
#[derive(Debug)]
pub struct RevChain<T, U> {
    first: T,
    second: U,
    done_first: bool,
}

impl<T, U> RevChain<T, U> {
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

impl<T: RevRead, U: RevRead> RevRead for RevChain<T, U> {
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

impl<T: RevBufRead, U: RevBufRead> RevBufRead for RevChain<T, U> {
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

/// An iterator over the contents of an instance of `RevBufRead` split on a
/// particular byte.
///
/// This struct is generally created by calling [`rev_split`] on a `RevBufRead`.
/// Please see the documentation of [`rev_split`] for more details.
///
/// [`rev_split`]: RevBufRead::rev_split
#[derive(Debug)]
pub struct RevSplit<B> {
    buf: B,
    delim: u8,
}

impl<B: RevBufRead> Iterator for RevSplit<B> {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Result<Vec<u8>>> {
        let mut buf = Vec::new();
        match self.buf.rev_read_until(self.delim, &mut buf) {
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

impl<B: RevBufRead> Iterator for RevLines<B> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Result<String>> {
        let mut buf = String::new();
        match self.buf.rev_read_line(&mut buf) {
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
pub struct RevTake<T> {
    inner: T,
    limit: u64,
}

impl<T> RevTake<T> {
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

impl<T: RevRead> RevRead for RevTake<T> {
    fn rev_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(0);
        }

        let buf_len = buf.len();

        let max = cmp::min(buf_len as u64, self.limit) as usize;
        let n = self.inner.rev_read(&mut buf[buf_len - max..])?;
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
            // if we just use an `as` cast to convert, limit may wrap around on a 32 bit target
            let limit = cmp::min(self.limit, usize::MAX as u64) as usize;

            let buf_init_ref_len = buf.init_ref().len();
            let extra_init = cmp::min(limit as usize, buf_init_ref_len);

            // SAFETY: no uninit data is written to ibuf
            let init_buf = {
                let buf_capacity = buf.capacity();
                unsafe { &mut buf.as_mut()[buf_capacity - limit..] }
            };

            let mut sliced_buf: RevBorrowedBuf<'_> = init_buf.into();

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

impl<T: RevBufRead> RevBufRead for RevTake<T> {
    fn rev_fill_buf(&mut self) -> Result<&[u8]> {
        // Don't call into inner reader at all at EOF because it may still block
        if self.limit == 0 {
            return Ok(&[]);
        }

        let buf = self.inner.rev_fill_buf()?;
        let buf_len = buf.len();

        let cap = cmp::min(buf_len as u64, self.limit) as usize;
        Ok(&buf[buf_len - cap..])
    }

    fn rev_consume(&mut self, amt: usize) {
        // Don't let callers reset the limit by passing an overlarge value
        let amt = cmp::min(amt as u64, self.limit) as usize;
        self.limit -= amt as u64;
        self.inner.rev_consume(amt);
    }
}

/// == default implementations ==
pub fn default_rev_read_to_end<R: RevRead + ?Sized>(
    reader: &mut R,
    dest_buf: &mut Vec<u8>,
) -> Result<usize> {
    let mut buffers: Vec<Vec<u8>> = vec![];
    let mut curr_buffer: Vec<u8> = vec![0; DEFAULT_BUF_SIZE];

    let mut amount_read: usize = 0;

    loop {
        match reader.rev_read(curr_buffer.as_mut_slice()) {
            Ok(amount) => {
                println!("{}", amount);
                if amount == 0 {
                    let mut final_buf = Vec::with_capacity(amount_read + dest_buf.len());

                    for buffer in buffers.into_iter().rev() {
                        final_buf.extend_from_slice(&buffer);
                    }
                    final_buf.extend_from_slice(&dest_buf);
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

pub fn default_rev_read_buf<F>(read: F, mut cursor: RevBorrowedCursor<'_>) -> Result<()>
where
    F: FnOnce(&mut [u8]) -> Result<usize>,
{
    let n = read(cursor.ensure_init().init_mut())?;
    cursor.advance(n);
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

#[cfg(test)]
mod tests {
    use super::*;

    mod rev_read {
        use super::*;

        mod rev_read_to_end {
            use super::*;

            #[test]
            fn general() {
                let data: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &data;
                let mut buffer = Vec::new();

                assert_eq!(reference.rev_read_to_end(&mut buffer).ok(), Some(3));
                assert!(reference.is_empty());
                assert_eq!(&buffer, &data);
            }
        }

        mod rev_read_to_string {
            use super::*;

            #[test]
            fn empty_data() {
                let data = b"";
                let mut string = String::new();

                assert_eq!(
                    data.as_slice().rev_read_to_string(&mut string).ok(),
                    Some(0)
                );
            }

            #[test]
            fn general() {
                let data = b"I use Arch btw.";

                let mut buffer = "Hi! ".to_string();
                assert_eq!(
                    data.as_slice().rev_read_to_string(&mut buffer).ok(),
                    Some(data.len())
                );
                assert_eq!(&buffer, "Hi! I use Arch btw.");
            }
        }

        mod rev_read_exact {
            use super::*;

            #[test]
            fn empty_buffer() {
                let data: [u8; 3] = [1, 2, 3];
                let mut buffer: [u8; 0] = [];

                assert!(data.as_slice().rev_read_exact(&mut buffer).is_ok());
            }

            #[test]
            fn buffer_bigger_than_data() {
                let data: [u8; 3] = [1, 2, 3];
                let mut buffer: [u8; 4] = [0; 4];

                let result = data.as_slice().rev_read_exact(&mut buffer);
                assert!(result.is_err());
                let err = result.unwrap_err();
                assert_eq!(err.kind(), ErrorKind::UnexpectedEof);
            }

            #[test]
            fn general() {
                let data: [u8; 3] = [1, 2, 3];
                let mut buffer: [u8; 2] = [0; 2];

                assert!(data.as_slice().rev_read_exact(&mut buffer).is_ok());
                assert_eq!(&buffer, &[2, 3]);
            }
        }

        mod rev_read_buf_exact {
            use super::*;

            // `data.as_slice().read_buf_exact(cursor).unwrap()` panics as well
            #[test]
            #[should_panic]
            fn empty_data() {
                let data: [u8; 0] = [];
                let mut buffer: [u8; 3] = [0; 3];

                let mut buf = RevBorrowedBuf::from(buffer.as_mut_slice());
                let cursor = buf.unfilled();

                assert!(data.as_slice().rev_read_buf_exact(cursor).is_ok());
            }

            #[test]
            fn buffer_smaller_than_data() {
                let data: [u8; 3] = [1, 2, 3];
                let mut buffer: [u8; 2] = [0; 2];

                let mut buf = RevBorrowedBuf::from(buffer.as_mut_slice());
                let cursor = buf.unfilled();

                assert!(data.as_slice().rev_read_buf_exact(cursor).is_ok());
                assert_eq!(&buffer, &[2, 3]);
            }

            // `data.as_slice().read_buf_exact(cursor).unwrap()` panics as well
            #[test]
            #[should_panic]
            fn buffer_bigger_than_data() {
                let data: [u8; 3] = [1, 2, 3];
                let mut buffer: [u8; 4] = [0; 4];

                let mut buf = RevBorrowedBuf::from(buffer.as_mut_slice());
                let cursor = buf.unfilled();

                data.as_slice().rev_read_buf_exact(cursor).unwrap();
            }
        }

        mod rev_take {
            use super::*;

            mod rev_read {
                use super::*;

                #[test]
                fn zero_rev_take() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 3] = [0, 0, 0];
                    let mut take = data.as_slice().rev_take(0);

                    assert_eq!(take.rev_read(&mut buffer).ok(), Some(0));
                    assert_eq!(&buffer, &[0, 0, 0]);
                }

                #[test]
                fn middle_rev_take() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 2] = [0, 0];
                    let mut take = data.as_slice().rev_take(1);

                    assert_eq!(take.rev_read(&mut buffer).ok(), Some(1));
                    assert_eq!(&buffer, &[0, 3]);
                }

                #[test]
                fn fill_rev_take() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 4] = [0; 4];
                    let mut take = data.as_slice().rev_take(data.len() as u64);

                    assert_eq!(take.rev_read(&mut buffer).ok(), Some(data.len()));
                    assert_eq!(&buffer, &[0, 1, 2, 3]);
                }
            }

            mod rev_read_buf {
                use super::*;

                #[test]
                fn zero_rev_read_buf() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 3] = [0; 3];

                    let mut take = data.as_slice().rev_take(0);

                    let mut buf = RevBorrowedBuf::from(buffer.as_mut_slice());
                    let cursor = buf.unfilled();

                    assert!(take.rev_read_buf(cursor).is_ok());
                    assert_eq!(&buffer, &[0; 3]);
                }

                #[test]
                fn full_rev_read_buf() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 3] = [0; 3];

                    let data_len = data.len();
                    let mut take = data.as_slice().rev_take(data_len as u64);

                    let mut buf = RevBorrowedBuf::from(buffer.as_mut_slice());
                    let cursor = buf.unfilled();

                    assert!(take.rev_read_buf(cursor).is_ok());
                    assert_eq!(&buffer, &data);
                }

                #[test]
                fn take_bigger_than_datta() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 3] = [0; 3];

                    let data_len = data.len();
                    let mut take = data.as_slice().rev_take((data_len + 10) as u64);

                    let mut buf = RevBorrowedBuf::from(buffer.as_mut_slice());
                    let cursor = buf.unfilled();

                    assert!(take.rev_read_buf(cursor).is_ok());
                    assert_eq!(&buffer, &data);
                }
            }
        }
    }

    mod rev_buf_read {
        use super::*;

        mod rev_read_until {
            use super::*;

            #[test]
            fn until_end() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut buffer = vec![];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.rev_read_until(0, &mut buffer).ok(), Some(3));
                assert!(reference.is_empty());
                assert_eq!(&buffer, &[1, 2, 3]);
            }

            #[test]
            fn delim_in_between() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut buffer = vec![];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.rev_read_until(2, &mut buffer).ok(), Some(2));
                assert_eq!(reference, &[1]);
                assert_eq!(&buffer, &[2, 3]);
            }

            #[test]
            fn delim_at_the_beginning() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut buffer = vec![];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.rev_read_until(3, &mut buffer).ok(), Some(1));
                assert_eq!(reference, &[1, 2]);
                assert_eq!(&buffer, &[3]);
            }
        }

        mod rev_skip_until {
            use super::*;

            #[test]
            fn until_end() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.rev_skip_until(0).ok(), Some(3));
                assert!(reference.is_empty());
            }

            #[test]
            fn delim_in_between() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.rev_skip_until(2).ok(), Some(2));
                assert_eq!(reference, &[1])
            }

            #[test]
            fn delim_at_the_beginning() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.rev_skip_until(3).ok(), Some(1));
                assert_eq!(reference, &[1, 2]);
            }
        }

        mod rev_read_line {
            use super::*;

            #[test]
            fn no_new_line() {
                let data = b"I use Arch btw.";
                let mut buffer = String::new();

                assert_eq!(
                    data.as_slice().rev_read_line(&mut buffer).ok(),
                    Some(data.len())
                );
                assert_eq!(buffer.as_bytes(), data as &[u8]);
            }

            #[test]
            fn new_line_in_between() {
                let data = b"first line\r\nsecond line";
                let mut buffer = String::new();

                assert_eq!(data.as_slice().rev_read_line(&mut buffer).ok(), Some(13));
                assert_eq!(&buffer, &"\r\nsecond line");
            }

            #[test]
            fn new_line_in_beginning() {
                let data = b"\nsus";
                let mut buffer = String::new();

                assert_eq!(data.as_slice().rev_read_line(&mut buffer).ok(), Some(4));
                assert_eq!(buffer.as_bytes(), data);
            }
        }

        mod rev_split {
            use super::*;

            #[test]
            fn no_delim() {
                let data = b"hello there";
                let mut split = data.as_slice().rev_split(b'k');

                let next = split.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                let next = next.unwrap().unwrap();

                assert_eq!(
                    next,
                    data.to_vec(),
                    "next: {}",
                    String::from_utf8(next.clone()).unwrap()
                );

                assert!(split.next().is_none());
            }

            #[test]
            fn delim_in_between() {
                let data = b"hello there";
                let mut split = data.as_slice().rev_split(b' ');

                let first = split.next();
                assert!(first.as_ref().is_some());
                assert!(first.as_ref().unwrap().is_ok());
                let first = first.unwrap().unwrap();

                let second = split.next();
                assert!(second.as_ref().is_some());
                assert!(second.as_ref().unwrap().is_ok());
                let second = second.unwrap().unwrap();

                assert_eq!(
                    first,
                    b"there".to_vec(),
                    "first: '{}'",
                    String::from_utf8(first.clone()).unwrap()
                );
                assert_eq!(
                    second,
                    b"hello".to_vec(),
                    "second: '{}'",
                    String::from_utf8(second.clone()).unwrap()
                );

                assert!(split.next().is_none());
            }
        }

        mod rev_lines {
            use super::*;

            #[test]
            fn no_new_lines() {
                let data = b"hello\rthere";
                let mut lines = data.as_slice().rev_lines();

                let next = lines.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                assert_eq!(
                    next.unwrap().unwrap(),
                    String::from_utf8(data.to_vec()).unwrap()
                );

                assert!(lines.next().is_none());
            }

            #[test]
            fn one_new_line_char() {
                let data = b"Hello there!\r\nGeneral kenobi!";
                let mut lines = data.as_slice().rev_lines();

                let next = lines.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                assert_eq!(next.unwrap().unwrap(), "General kenobi!".to_string());

                let next = lines.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                assert_eq!(next.unwrap().unwrap(), "Hello there!".to_string());

                assert!(lines.next().is_none());
            }
        }

        mod rev_take {
            use super::*;

            mod rev_fill_buf {
                use super::*;

                #[test]
                fn middle_rev_fill_buf() {
                    let data: [u8; 3] = [1, 2, 3];

                    let mut take = data.as_slice().rev_take(2);

                    let buf = take.rev_fill_buf();
                    assert_eq!(buf.ok(), Some([2, 3].as_slice()));
                }

                #[test]
                fn exceeding_take_value() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut take = data.as_slice().rev_take((data.len() + 10) as u64);

                    let buf = take.rev_fill_buf();
                    assert_eq!(buf.ok(), Some(data.as_slice()));
                }
            }

            mod rev_consume {
                use super::*;

                #[test]
                fn exceeding_consume() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut take = data.as_slice().rev_take(data.len() as u64);
                    take.rev_consume(1);

                    assert_eq!(take.rev_fill_buf().ok(), Some([1, 2].as_slice()));
                }
            }
        }

        mod rev_chain {
            use super::*;

            #[test]
            fn empty_chain() {
                let data1: [u8; 0] = [];
                let data2: [u8; 0] = [];

                let mut buffer: Vec<u8> = Vec::new();

                let mut rev_chain = data1.as_slice().rev_chain(data2.as_slice());

                assert_eq!(rev_chain.rev_read(&mut buffer).ok(), Some(0));
                assert!(buffer.is_empty());
            }

            #[test]
            fn first_chain_full() {
                let data1: [u8; 3] = [1, 2, 3];
                let data2: [u8; 0] = [];

                let mut buffer: [u8; 4] = [0; 4];

                let mut rev_chain = data1.as_slice().rev_chain(data2.as_slice());

                assert_eq!(rev_chain.rev_read(&mut buffer).ok(), Some(3));
                assert_eq!(&buffer, &[0, 1, 2, 3]);
            }
        }

        mod rev_bytes {
            use super::*;

            #[test]
            fn empty_data() {
                let data: [u8; 0] = [];

                let mut rev_bytes = data.as_slice().rev_bytes();
                assert!(rev_bytes.next().is_none());
            }

            #[test]
            fn general() {
                let data: [u8; 3] = [1, 2, 3];

                let mut rev_bytes = data.as_slice().rev_bytes();
                for byte_value in 3..=1 {
                    let next_value = rev_bytes.next();

                    assert!(&next_value.is_some());
                    assert!(next_value.as_ref().unwrap().is_ok());
                    assert_eq!(next_value.unwrap().unwrap(), byte_value);
                }
            }
        }
    }
}
