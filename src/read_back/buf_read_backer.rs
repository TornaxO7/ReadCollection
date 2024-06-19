use crate::{BufReadBack, ReadBack};

#[derive(Debug)]
pub struct BufReadBacker {}

impl ReadBack for BufReadBacker {
    fn read_back(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        todo!()
    }
}

impl BufReadBack for BufReadBacker {
    fn read_back_fill_buf(&mut self) -> std::io::Result<&[u8]> {
        todo!()
    }

    fn read_back_consume(&mut self, amt: usize) {
        todo!()
    }
}
