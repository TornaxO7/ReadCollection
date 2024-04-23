use std::io::{self, IoSliceMut, Result, Take};

use crate::{rev_borrowed_buf::RevBorrowedCursor, RevBufRead};

use self::{
    defaults::{
        default_rev_read_buf, default_rev_read_buf_exact, default_rev_read_exact,
        default_rev_read_to_end, default_rev_read_to_string, default_rev_read_vectored,
    },
    utils::Bytes,
};

mod defaults;
mod impls;
mod utils;

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
    fn rev_read_buf(&mut self, buf: RevBorrowedCursor<'_>) -> Result<()> {
        default_rev_read_buf(|b| self.rev_read(b), buf)
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
        todo!("can't directly use from std")
    }
    fn rev_take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        todo!("Can't directly use from std");
    }
}
