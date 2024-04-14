use std::io::{self, Lines, Split};

use crate::RevRead;

pub trait RevBufRead: RevRead {
    fn rev_fill_buf(&mut self) -> io::Result<&[u8]>;
    fn rev_consume(&mut self, amt: usize);

    // Provided methods
    fn rev_has_data_left(&mut self) -> io::Result<bool> {
        todo!()
    }

    fn rev_read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> io::Result<usize> {
        todo!()
    }

    fn rev_skip_until(&mut self, byte: u8) -> io::Result<usize> {
        todo!()
    }

    fn rev_read_line(&mut self, buf: &mut String) -> io::Result<usize> {
        todo!()
    }

    fn rev_split(self, byte: u8) -> Split<Self>
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
