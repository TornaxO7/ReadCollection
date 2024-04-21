use std::io::{self, BorrowedCursor, Bytes, Chain, IoSliceMut, Take};

mod impls;

/// Equals the [std::io::Read] trait, except that everything is in reverse.
pub trait RevRead {
    fn rev_read(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    fn rev_read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        todo!()
    }
    fn rev_is_read_vectored(&self) -> bool {
        todo!()
    }
    fn rev_read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        todo!()
    }
    fn rev_read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        todo!()
    }
    fn rev_read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        todo!()
    }
    fn rev_read_buf(&mut self, buf: BorrowedCursor<'_>) -> io::Result<()> {
        todo!()
    }
    fn rev_read_buf_exact(&mut self, cursor: BorrowedCursor<'_>) -> io::Result<()> {
        todo!()
    }
    fn rev_by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        todo!()
    }
    fn rev_bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        todo!()
    }
    fn rev_chain<R: io::Read>(self, next: R) -> Chain<Self, R>
    where
        Self: Sized,
    {
        todo!()
    }
    fn rev_take(self, limit: u64) -> Take<Self>
    where
        Self: Sized,
    {
        todo!()
    }
}
