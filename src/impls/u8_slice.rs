use std::io::IoSliceMut;
use std::{cmp, io::Read};

use crate::BufReadBack;
use crate::ReadBack;

/// As for the [`Read`] implementation of `&[u8]`, bytes get copied from the slice.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html#impl-Read-for-%26%5Bu8%5D
impl ReadBack for &[u8] {
    fn read_back(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let buf_len = buf.len();
        let self_len = self.len();

        let amount = cmp::min(buf_len, self_len);
        let (tail, head) = self.split_at(self_len - amount);

        if amount == 1 {
            // SAFETY:
            //  - If amount == 1 == buf.len(), then there's at least one value!
            //  - If buf.len() < 1 => amount < 1 as well => not possible
            //  - otherwise buf.len() > 1
            let buf_last = buf.last_mut().unwrap();
            // SAFETY:
            //  - If amount == 1 == self.len(), then `tail` would be empty and `head` would get the value
            //  - If self.len() < 1 => amount < 1 as well => not possible
            //  - otherwise self.len() > 1
            let head_last = head.last().unwrap();

            *buf_last = *head_last;
        } else {
            buf[buf_len - amount..].copy_from_slice(head);
        }

        *self = tail;

        Ok(amount)
    }

    fn read_back_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        let mut amount_read = 0;
        for buf in bufs {
            amount_read += self.read_back(buf)?;
            if self.is_empty() {
                break;
            }
        }

        Ok(amount_read)
    }

    fn read_back_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        let len = self.len();
        buf.try_reserve(len)
            .map_err(|_| std::io::ErrorKind::OutOfMemory)?;

        let mut new_vec = self.to_vec();
        new_vec.extend_from_slice(buf);
        *buf = new_vec;

        *self = &[];

        Ok(len)
    }

    fn read_back_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        // validating the bytes from right to left or left to right doesn't differ
        self.read_to_string(buf)
    }

    fn read_back_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        if buf.len() > self.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "failed to fill whole buffer",
            ));
        }

        let (tail, head) = self.split_at(self.len() - buf.len());

        if buf.len() == 1 {
            let last_buf_value = buf.last_mut().unwrap();
            *last_buf_value = *head.last().unwrap();
        } else {
            let head_len = head.len();
            buf.copy_from_slice(&head[head_len - buf.len()..]);
        }

        *self = tail;

        Ok(())
    }
}

impl BufReadBack for &[u8] {
    fn read_back_fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(*self)
    }

    fn read_back_consume(&mut self, amt: usize) {
        let end = self.len().saturating_sub(amt);
        *self = &self[..end];
    }
}

#[cfg(test)]
mod tests {
    use super::ReadBack;

    mod rev_read {
        use super::ReadBack;

        #[test]
        fn amount_1() {
            let values = [1, 2, 3];
            let mut buffer = [0];

            assert_eq!(values.as_slice().read_back(&mut buffer).ok(), Some(1));
            assert_eq!(buffer, [3]);
        }

        #[test]
        fn multiple() {
            let values = [1, 2, 3];
            let mut buffer = [0, 0];

            assert_eq!(values.as_slice().read_back(&mut buffer).ok(), Some(2));
            assert_eq!(buffer, [2, 3]);
        }

        #[test]
        fn bigger_buffer_than_self() {
            let values = [1, 2, 3];
            let mut buffer = [0, 0, 0, 0];

            assert_eq!(values.as_slice().read_back(&mut buffer).ok(), Some(3));
            assert_eq!(buffer, [1, 2, 3, 0]);
        }
    }

    mod rev_read_exact {
        use super::ReadBack;
        #[test]
        fn empty_buf() {
            let values = [1, 2, 3];
            let mut buffer = [];

            assert!(values.as_slice().read_back_exact(&mut buffer).is_ok());
        }

        #[test]
        fn normal() {
            let values = [1, 2, 3];
            let mut buffer = [0, 0];

            assert!(values.as_slice().read_back_exact(&mut buffer).is_ok());
            assert_eq!(buffer, [2, 3]);
        }

        #[test]
        fn equal_size() {
            let values = [1, 2, 3];
            let mut buffer = [0, 0, 0];

            assert!(values.as_slice().read_back_exact(&mut buffer).is_ok());
            assert_eq!(buffer, [1, 2, 3]);
        }
    }

    mod rev_read_to_end {
        use super::ReadBack;
        #[test]
        fn empty_vec() {
            let values = [1, 2, 3];
            let mut buffer = vec![];

            assert_eq!(
                values.as_slice().read_back_to_end(&mut buffer).ok(),
                Some(3)
            );
            assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        }

        #[test]
        fn non_empty_vec() {
            let values = [1, 2, 3];
            let mut buffer = vec![4];

            assert_eq!(
                values.as_slice().read_back_to_end(&mut buffer).ok(),
                Some(3)
            );
            assert_eq!(buffer.as_slice(), &[1, 2, 3, 4]);
        }
    }

    mod rev_buf_read {
        use crate::BufReadBack;

        #[test]
        fn rev_consume_large_amt() {
            let values: [u8; 3] = [1, 2, 3];
            let mut reference: &[u8] = &values;

            reference.read_back_consume(values.len() + 1);
            assert!(reference.is_empty());
        }
    }
}
