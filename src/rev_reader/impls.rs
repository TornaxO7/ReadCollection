use crate::RevRead;

impl RevRead for &[u8] {
    fn rev_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let buf_len = buf.len();
        let offset = std::cmp::min(self.len(), buf_len);

        let (head, tail) = self.split_at(self.len() - offset);

        if offset == 1 {
            if let (Some(a), Some(b)) = (buf.last_mut(), tail.last()) {
                *a = *b;
            }
        } else {
            buf[buf_len - offset..].copy_from_slice(tail);
        }

        *self = head;
        Ok(offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod u8_slice {
        use super::*;

        #[test]
        fn basic() {
            let data = [1, 2, 3];
            let mut buffer = [0, 0];

            assert_eq!(data.as_slice().rev_read(&mut buffer).ok(), Some(2));
            assert_eq!(buffer, [2, 3]);
        }

        #[test]
        fn empty_slice() {
            let data: [u8; 0] = [];
            let mut buffer = [0, 0];

            assert_eq!(data.as_slice().rev_read(&mut buffer).ok(), Some(0));
            assert_eq!(buffer, [0, 0]);
        }
    }
}
