use std::{
    fs::File,
    io::{Read, Seek},
};

use crate::RevRead;

impl RevRead for File {
    fn rev_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let curr_pos = self.stream_position()?;

        // you shouldn't use buf.len() here....
        let buf_len = buf.len() as u64;
        let max_amount_read = std::cmp::min(curr_pos, buf_len);

        self.seek(std::io::SeekFrom::Current(-(max_amount_read as i64)))?;
        let (_left, right) = buf.split_at_mut((buf_len - max_amount_read) as usize);
        match self.read(right) {
            Ok(n) => {
                self.seek(std::io::SeekFrom::Current(
                    (max_amount_read - (n as u64)) as i64,
                ))?;
                return Ok(n);
            }
            Err(err) => return Err(err),
        }
    }
}
