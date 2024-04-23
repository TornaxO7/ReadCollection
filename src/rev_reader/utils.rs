use std::{
    io::{ErrorKind, IoSliceMut, Result},
    slice,
};

use crate::{rev_borrowed_buf::RevBorrowedCursor, RevBufRead, RevRead};

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

// impl<T> SizeHint for &mut T {
//     #[inline]
//     fn lower_bound(&self) -> usize {
//         SizeHint::lower_bound(*self)
//     }

//     #[inline]
//     fn upper_bound(&self) -> Option<usize> {
//         SizeHint::upper_bound(*self)
//     }
// }

// impl<T> SizeHint for Box<T> {
//     #[inline]
//     fn lower_bound(&self) -> usize {
//         SizeHint::lower_bound(&**self)
//     }

//     #[inline]
//     fn upper_bound(&self) -> Option<usize> {
//         SizeHint::upper_bound(&**self)
//     }
// }

// impl SizeHint for &[u8] {
//     #[inline]
//     fn lower_bound(&self) -> usize {
//         self.len()
//     }

//     #[inline]
//     fn upper_bound(&self) -> Option<usize> {
//         Some(self.len())
//     }
// }

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
                buf if buf.is_empty() => self.done_first = true,
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

    // We don't override `read_line` here because an UTF-8 sequence could be
    // split between the two parts of the chain
}

impl<T, U> SizeHint for Chain<T, U> {
    #[inline]
    fn lower_bound(&self) -> usize {
        SizeHint::lower_bound(&self.first) + SizeHint::lower_bound(&self.second)
    }

    #[inline]
    fn upper_bound(&self) -> Option<usize> {
        match (
            SizeHint::upper_bound(&self.first),
            SizeHint::upper_bound(&self.second),
        ) {
            (Some(first), Some(second)) => first.checked_add(second),
            _ => None,
        }
    }
}
