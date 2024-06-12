use std::io::Empty;

use crate::{BufReadBack, ReadBack};

impl ReadBack for Empty {
    fn read_back(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

impl BufReadBack for Empty {
    fn read_back_fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(&[])
    }

    fn read_back_consume(&mut self, _amt: usize) {}
}
