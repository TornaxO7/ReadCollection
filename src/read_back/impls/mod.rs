use crate::{BufReadBack, ReadBack};

mod empty;
mod file;
mod u8_slice;

impl<R: ReadBack> ReadBack for &mut R {
    fn read_back(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        (**self).read_back(buf)
    }

    fn read_back_vectored(
        &mut self,
        bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> std::io::Result<usize> {
        (**self).read_back_vectored(bufs)
    }

    fn read_back_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        (**self).read_back_to_end(buf)
    }

    fn read_back_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        (**self).read_back_to_string(buf)
    }

    fn read_back_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        (**self).read_back_exact(buf)
    }
}

impl<R: ReadBack> ReadBack for Box<R> {
    fn read_back(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        (**self).read_back(buf)
    }

    fn read_back_vectored(
        &mut self,
        bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> std::io::Result<usize> {
        (**self).read_back_vectored(bufs)
    }

    fn read_back_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        (**self).read_back_to_end(buf)
    }

    fn read_back_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        (**self).read_back_to_string(buf)
    }

    fn read_back_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        (**self).read_back_exact(buf)
    }
}

impl<R: BufReadBack> BufReadBack for &mut R {
    fn read_back_fill_buf(&mut self) -> std::io::Result<&[u8]> {
        (**self).read_back_fill_buf()
    }

    fn read_back_consume(&mut self, amt: usize) {
        (**self).read_back_consume(amt)
    }

    fn read_back_has_data_left(&mut self) -> std::io::Result<bool> {
        (**self).read_back_has_data_left()
    }

    fn read_back_until(&mut self, delim: u8, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        (**self).read_back_until(delim, buf)
    }

    fn read_back_skip_until(&mut self, delim: u8) -> std::io::Result<usize> {
        (**self).read_back_skip_until(delim)
    }

    fn read_back_line(&mut self, dest: &mut String) -> std::io::Result<usize> {
        (**self).read_back_line(dest)
    }
}

impl<R: BufReadBack> BufReadBack for Box<R> {
    fn read_back_fill_buf(&mut self) -> std::io::Result<&[u8]> {
        (**self).read_back_fill_buf()
    }

    fn read_back_consume(&mut self, amt: usize) {
        (**self).read_back_consume(amt)
    }

    fn read_back_has_data_left(&mut self) -> std::io::Result<bool> {
        (**self).read_back_has_data_left()
    }

    fn read_back_until(&mut self, delim: u8, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        (**self).read_back_until(delim, buf)
    }

    fn read_back_skip_until(&mut self, delim: u8) -> std::io::Result<usize> {
        (**self).read_back_skip_until(delim)
    }

    fn read_back_line(&mut self, dest: &mut String) -> std::io::Result<usize> {
        (**self).read_back_line(dest)
    }
}
