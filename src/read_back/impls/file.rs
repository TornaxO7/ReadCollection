use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use crate::ReadBack;

impl ReadBack for &File {
    fn read_back(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let curr_pos = self.stream_position()?;

        let buf_len = buf.len() as u64;
        let max_amount_read = std::cmp::min(curr_pos, buf_len);

        self.seek(SeekFrom::Current(-(max_amount_read as i64)))?;
        let (_left, right) = buf.split_at_mut((buf_len - max_amount_read) as usize);
        match self.read(right) {
            Ok(n) => {
                let offset = std::cmp::min(max_amount_read, n as u64) as i64;
                self.seek(std::io::SeekFrom::Current(-offset))?;
                Ok(n)
            }
            Err(err) => Err(err),
        }
    }
}

impl ReadBack for File {
    fn read_back(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        (&*self).read_back(buf)
    }
}
