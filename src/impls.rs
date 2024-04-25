use std::cmp;
use std::io::IoSliceMut;

use crate::{rev_borrowed_buf::RevBorrowedCursor, RevRead};

impl RevRead for &[u8] {
    fn rev_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
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

    fn rev_read_buf(&mut self, mut cursor: RevBorrowedCursor<'_>) -> std::io::Result<()> {
        let amount = cmp::min(cursor.capacity(), self.len());
        let (tail, head) = self.split_at(self.len() - amount);

        cursor.append(head);

        *self = tail;
        Ok(())
    }

    fn rev_read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        todo!();
    }

    fn rev_is_read_vectored(&self) -> bool {
        true
    }

    fn rev_read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        todo!()
    }

    fn rev_read_buf_exact(&mut self, cursor: RevBorrowedCursor<'_>) -> std::io::Result<()> {
        todo!();
    }

    fn rev_read_to_string(&mut self, buf: &mut String) -> std::io::Result<usize> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod rev_read {
        use super::*;

        #[test]
        fn amount_1() {
            let values = [1, 2, 3];
            let mut buffer = [0];

            assert_eq!(values.as_slice().rev_read(&mut buffer).ok(), Some(1));
            assert_eq!(buffer, [3]);
        }

        #[test]
        fn multiple() {
            let values = [1, 2, 3];
            let mut buffer = [0, 0];

            assert_eq!(values.as_slice().rev_read(&mut buffer).ok(), Some(2));
            assert_eq!(buffer, [2, 3]);
        }

        #[test]
        fn bigger_buffer_than_self() {
            let values = [1, 2, 3];
            let mut buffer = [0, 0, 0, 0];

            assert_eq!(values.as_slice().rev_read(&mut buffer).ok(), Some(3));
            assert_eq!(buffer, [0, 1, 2, 3]);
        }
    }

    mod rev_read_buf {
        use crate::RevBorrowedBuf;

        use super::*;

        #[test]
        fn empty_cursor() {
            let values: [u8; 3] = [1, 2, 3];
            let mut buffer: [u8; 0] = [];
            let mut borrowed_buf = RevBorrowedBuf::from(buffer.as_mut_slice());
            let cursor = borrowed_buf.unfilled();

            assert!(values.as_slice().rev_read_buf(cursor).is_ok());
        }

        #[test]
        fn normal_cursor() {
            let values: [u8; 3] = [1, 2, 3];
            let mut buffer: [u8; 2] = [0, 0];
            let mut borrowed_buf = RevBorrowedBuf::from(buffer.as_mut_slice());
            let cursor = borrowed_buf.unfilled();

            assert!(values.as_slice().rev_read_buf(cursor).is_ok());
            assert_eq!(buffer, [2, 3]);
        }

        #[test]
        fn cursor_longer_than_values() {
            let values: [u8; 3] = [1, 2, 3];
            let mut buffer: [u8; 4] = [0, 0, 0, 0];
            let mut borrowed_buf = RevBorrowedBuf::from(buffer.as_mut_slice());
            let cursor = borrowed_buf.unfilled();

            assert!(values.as_slice().rev_read_buf(cursor).is_ok());
            assert_eq!(buffer, [0, 1, 2, 3]);
        }
    }
}
