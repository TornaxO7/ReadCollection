use std::io::{self, BufRead, Read, Seek};

use crate::{RevBufRead, RevRead};

use self::buffer::Buffer;

mod buffer;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
pub const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

pub struct BiBufReader<R> {
    buf: Buffer,
    inner: R,
}

impl<R> BiBufReader<R> {
    pub fn capacity(&self) -> usize {
        self.buf.capacity()
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    pub fn into_inner(self) -> R
    where
        R: Sized,
    {
        self.inner
    }

    pub fn new(inner: R) -> Self {
        Self::with_capacity(DEFAULT_BUF_SIZE, inner)
    }

    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Self {
            buf: Buffer::with_capacity(capacity),
            inner,
        }
    }
}

impl<R: Read> Read for BiBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let nothing_buffered = self.buf.pos() == self.buf.filled();
        let buf_exceeds_internal_buffer = buf.len() >= self.capacity();

        if nothing_buffered && buf_exceeds_internal_buffer {
            self.buf.discard_buffer();
            return self.inner.read(buf);
        }

        let mut added_content = self.fill_buf()?;
        let amount_read = added_content.read(buf)?;
        self.consume(amount_read);
        Ok(amount_read)
    }
}

impl<R: Read> BufRead for BiBufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.buf.fill_buf(&mut self.inner)
    }

    fn consume(&mut self, amt: usize) {
        self.buf.consume(amt);
    }
}

impl<R: Read + Seek> RevRead for BiBufReader<R> {
    fn rev_read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let curr_pos = self.inner.stream_position()?;
        let nothing_buffered = self.buf.pos() == 0;
        let buf_exceeds_internal_buffer = buf.len() >= self.capacity();
        if nothing_buffered && buf_exceeds_internal_buffer {
            // big read into the provided buffer, since we can't even buffer the big read
            let offset = std::cmp::max(-(curr_pos as i64), -(buf.len() as i64));

            self.inner.seek(io::SeekFrom::Current(offset))?;
            return self.inner.read(buf);
        }

        let internal_buffer_is_buffer_reuseable = self.buf.pos() >= buf.len();
        if internal_buffer_is_buffer_reuseable {
            // reuse the content of the internal buffer
            let internal_buffer_content = self.buf.rev_buffer();

            let mut relevant_part =
                &internal_buffer_content[self.buf.pos().saturating_sub(buf.len())..self.buf.pos()];
            let amount_read = relevant_part.read(buf)?;
            self.rev_consume(amount_read);
            return Ok(amount_read);
        }

        // otherwise: buffer the content on the left from the current position
        self.buf.discard_buffer();
        let added_content = self.rev_fill_buf()?;
        debug_assert!(buf.len() <= added_content.len());
        let start = added_content.len().saturating_sub(buf.len());
        let mut relevant_part = &added_content[start..];
        let amount_read = relevant_part.read(buf)?;
        self.rev_consume(amount_read);
        Ok(amount_read)
    }
}

impl<R: Read + Seek> RevBufRead for BiBufReader<R> {
    fn rev_fill_buf(&mut self) -> io::Result<&[u8]> {
        self.buf.rev_fill_buf(&mut self.inner)
    }

    fn rev_consume(&mut self, amt: usize) {
        self.buf.rev_consume(amt)
    }
}

impl<R: Seek> Seek for BiBufReader<R> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     const DATA: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
//     const CURSOR_DATA: io::Cursor<&[u8; 10]> = io::Cursor::new(&DATA);

//     fn get_reader() -> BiBufReader<io::Cursor<&'static [u8; 10]>> {
//         BiBufReader::new(CURSOR_DATA)
//     }

//     mod bibufreader_equals_bufreader {
//         use std::io::BufReader;

//         use super::*;

//         #[test]
//         fn with_elements() {
//             let mut tester = BufReader::new(CURSOR_DATA);
//             let mut reader = get_reader();
//             let mut buf1 = [0, 0, 0];
//             let mut buf2 = [0, 0, 0];

//             assert_eq!(reader.read(&mut buf1).ok(), tester.read(&mut buf2).ok());
//             assert_eq!(buf1, buf2);

//             assert_eq!(reader.read(&mut buf1).ok(), tester.read(&mut buf2).ok());
//             assert_eq!(buf1, buf2);
//         }

//         #[test]
//         fn with_empty_data() {
//             let data: Vec<u8> = vec![];
//             let mut tester = BufReader::new(data.as_slice());
//             let mut reader = BiBufReader::new(data.as_slice());

//             let mut buf1 = [0, 0, 0];
//             let mut buf2 = [0, 0, 0];

//             assert_eq!(reader.read(&mut buf1).ok(), tester.read(&mut buf2).ok());
//             assert_eq!(buf1, buf2);
//         }

//         #[test]
//         fn with_discarding() {
//             let data: Vec<u8> = vec![1, 2, 3];
//             let mut tester = BufReader::new(data.as_slice());
//             let mut reader = BiBufReader::new(data.as_slice());

//             let mut buf1 = [0, 0];
//             let mut buf2 = [0, 0];

//             assert_eq!(reader.read(&mut buf1).ok(), tester.read(&mut buf2).ok());
//             assert_eq!(buf1, [1, 2]);
//             assert_eq!(buf1, buf2);

//             // we discarded the buffer and add the next value to the first index
//             assert_eq!(reader.read(&mut buf1).ok(), tester.read(&mut buf2).ok());
//             assert_eq!(buf1, [3, 2]);
//             assert_eq!(buf1, buf2);
//         }
//     }

//     mod rev_read {
//         use super::*;

//         #[test]
//         fn with_elements() {
//             let mut reader = get_reader();
//             reader.seek(io::SeekFrom::End(0)).unwrap();
//             let mut buffer = [0, 0, 0];

//             assert_eq!(reader.rev_read(&mut buffer).ok(), Some(3));
//             assert_eq!(buffer, [7, 8, 9]);

//             assert_eq!(reader.rev_read(&mut buffer).ok(), Some(3));
//             assert_eq!(buffer, [4, 5, 6]);
//         }

//         #[test]
//         fn with_discarding() {
//             let data: Vec<u8> = vec![1, 2, 3];
//             let mut reader = BiBufReader::new(io::Cursor::new(data.as_slice()));
//             let mut buffer = [0, 0];

//             assert_eq!(reader.rev_read(&mut buffer).ok(), Some(2));
//             assert_eq!(buffer, [2, 3]);

//             assert_eq!(reader.rev_read(&mut buffer).ok(), Some(1));
//             assert_eq!(buffer, [2, 1]);
//         }
//     }

//     #[test]
//     fn read_and_rev_read_basic() {
//         let middle = DATA.len() / 2;
//         let mut reader = BiBufReader::new(CURSOR_DATA);
//         reader.seek(io::SeekFrom::Start(middle as u64)).unwrap();

//         let mut read_buffer = [0, 0, 0];
//         let mut rev_read_buffer = [0, 0, 0];

//         // read the next 3 values on the right from the middle
//         assert_eq!(reader.read(&mut read_buffer).ok(), Some(3));
//         assert_eq!(read_buffer, [5, 6, 7]);

//         // re-read the just 3 elements which we read
//         assert_eq!(reader.rev_read(&mut rev_read_buffer).ok(), Some(3));
//         assert_eq!(rev_read_buffer, [5, 6, 7]);
//     }
// }
