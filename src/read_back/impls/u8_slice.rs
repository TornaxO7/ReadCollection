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
    use super::*;

    mod read_back {
        use super::*;

        mod read_back {
            use super::*;

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

        mod read_back_to_end {
            use super::*;

            #[test]
            fn general() {
                let data: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &data;
                let mut buffer = Vec::new();

                assert_eq!(reference.read_back_to_end(&mut buffer).ok(), Some(3));
                assert!(reference.is_empty());
                assert_eq!(&buffer, &data);
            }

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

        mod read_back_to_string {
            use super::*;

            #[test]
            fn empty_data() {
                let data = b"";
                let mut string = String::new();

                assert_eq!(
                    data.as_slice().read_back_to_string(&mut string).ok(),
                    Some(0)
                );
            }

            #[test]
            fn general() {
                let data = b"I use Arch btw.";

                let mut buffer = "Hi! ".to_string();
                assert_eq!(
                    data.as_slice().read_back_to_string(&mut buffer).ok(),
                    Some(data.len())
                );
                assert_eq!(&buffer, "Hi! I use Arch btw.");
            }
        }

        mod read_back_exact {
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

        mod read_back_take {
            use super::*;

            mod read_back {
                use super::*;

                #[test]
                fn zero_rev_take() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 3] = [0, 0, 0];
                    let mut take = data.as_slice().read_back_take(0);

                    assert_eq!(take.read_back(&mut buffer).ok(), Some(0));
                    assert_eq!(&buffer, &[0, 0, 0]);
                }

                #[test]
                fn middle_rev_take() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 2] = [0, 0];
                    let mut take = data.as_slice().read_back_take(1);

                    assert_eq!(take.read_back(&mut buffer).ok(), Some(1));
                    assert_eq!(&buffer, &[0, 3]);
                }

                #[test]
                fn fill_rev_take() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut buffer: [u8; 4] = [0; 4];
                    let mut take = data.as_slice().read_back_take(data.len() as u64);

                    assert_eq!(take.read_back(&mut buffer).ok(), Some(data.len()));
                    assert_eq!(&buffer, &[0, 1, 2, 3]);
                }
            }
        }
    }

    mod buf_read_back {
        use super::*;

        #[test]
        fn rev_consume_large_amt() {
            let values: [u8; 3] = [1, 2, 3];
            let mut reference: &[u8] = &values;

            reference.read_back_consume(values.len() + 1);
            assert!(reference.is_empty());
        }

        mod read_back_until {
            use super::*;

            #[test]
            fn until_end() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut buffer = vec![];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.read_back_until(0, &mut buffer).ok(), Some(3));
                assert!(reference.is_empty());
                assert_eq!(&buffer, &[1, 2, 3]);
            }

            #[test]
            fn delim_in_between() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut buffer = vec![];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.read_back_until(2, &mut buffer).ok(), Some(2));
                assert_eq!(reference, &[1]);
                assert_eq!(&buffer, &[2, 3]);
            }

            #[test]
            fn delim_at_the_beginning() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut buffer = vec![];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.read_back_until(3, &mut buffer).ok(), Some(1));
                assert_eq!(reference, &[1, 2]);
                assert_eq!(&buffer, &[3]);
            }
        }

        mod read_back_skip_until {
            use super::*;

            #[test]
            fn until_end() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.read_back_skip_until(0).ok(), Some(3));
                assert!(reference.is_empty());
            }

            #[test]
            fn delim_in_between() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.read_back_skip_until(2).ok(), Some(2));
                assert_eq!(reference, &[1])
            }

            #[test]
            fn delim_at_the_beginning() {
                let haystack: [u8; 3] = [1, 2, 3];
                let mut reference: &[u8] = &haystack;

                assert_eq!(reference.read_back_skip_until(3).ok(), Some(1));
                assert_eq!(reference, &[1, 2]);
            }
        }

        mod read_back_line {
            use super::*;

            #[test]
            fn no_new_line() {
                let data = b"I use Arch btw.";
                let mut buffer = String::new();

                assert_eq!(
                    data.as_slice().read_back_line(&mut buffer).ok(),
                    Some(data.len())
                );
                assert_eq!(buffer.as_bytes(), data as &[u8]);
            }

            #[test]
            fn new_line_in_between() {
                let data = b"first line\r\nsecond line";
                let mut buffer = String::new();

                assert_eq!(data.as_slice().read_back_line(&mut buffer).ok(), Some(13));
                assert_eq!(&buffer, &"\r\nsecond line");
            }

            #[test]
            fn new_line_in_beginning() {
                let data = b"\nsus";
                let mut buffer = String::new();

                assert_eq!(data.as_slice().read_back_line(&mut buffer).ok(), Some(4));
                assert_eq!(buffer.as_bytes(), data);
            }
        }

        mod read_back_split {
            use super::*;

            #[test]
            fn no_delim() {
                let data = b"hello there";
                let mut split = data.as_slice().read_back_split(b'k');

                let next = split.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                let next = next.unwrap().unwrap();

                assert_eq!(
                    next,
                    data.to_vec(),
                    "next: {}",
                    String::from_utf8(next.clone()).unwrap()
                );

                assert!(split.next().is_none());
            }

            #[test]
            fn delim_in_between() {
                let data = b"hello there";
                let mut split = data.as_slice().read_back_split(b' ');

                let first = split.next();
                assert!(first.as_ref().is_some());
                assert!(first.as_ref().unwrap().is_ok());
                let first = first.unwrap().unwrap();

                let second = split.next();
                assert!(second.as_ref().is_some());
                assert!(second.as_ref().unwrap().is_ok());
                let second = second.unwrap().unwrap();

                assert_eq!(
                    first,
                    b"there".to_vec(),
                    "first: '{}'",
                    String::from_utf8(first.clone()).unwrap()
                );
                assert_eq!(
                    second,
                    b"hello".to_vec(),
                    "second: '{}'",
                    String::from_utf8(second.clone()).unwrap()
                );

                assert!(split.next().is_none());
            }
        }

        mod read_back_lines {
            use super::*;

            #[test]
            fn no_new_lines() {
                let data = b"hello\rthere";
                let mut lines = data.as_slice().read_back_lines();

                let next = lines.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                assert_eq!(
                    next.unwrap().unwrap(),
                    String::from_utf8(data.to_vec()).unwrap()
                );

                assert!(lines.next().is_none());
            }

            #[test]
            fn one_new_line_char() {
                let data = b"Hello there!\r\nGeneral kenobi!";
                let mut lines = data.as_slice().read_back_lines();

                let next = lines.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                assert_eq!(next.unwrap().unwrap(), "General kenobi!".to_string());

                let next = lines.next();
                assert!(next.as_ref().is_some());
                assert!(next.as_ref().unwrap().is_ok());
                assert_eq!(next.unwrap().unwrap(), "Hello there!".to_string());

                assert!(lines.next().is_none());
            }
        }

        mod read_back_take {
            use super::*;

            mod read_back_fill_buf {
                use super::*;

                #[test]
                fn middle_read_back_fill_buf() {
                    let data: [u8; 3] = [1, 2, 3];

                    let mut take = data.as_slice().read_back_take(2);

                    let buf = take.read_back_fill_buf();
                    assert_eq!(buf.ok(), Some([2, 3].as_slice()));
                }

                #[test]
                fn exceeding_take_value() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut take = data.as_slice().read_back_take((data.len() + 10) as u64);

                    let buf = take.read_back_fill_buf();
                    assert_eq!(buf.ok(), Some(data.as_slice()));
                }
            }

            mod read_back_consume {
                use super::*;

                #[test]
                fn exceeding_consume() {
                    let data: [u8; 3] = [1, 2, 3];
                    let mut take = data.as_slice().read_back_take(data.len() as u64);
                    take.read_back_consume(1);

                    assert_eq!(take.read_back_fill_buf().ok(), Some([1, 2].as_slice()));
                }
            }
        }

        mod read_back_chain {
            use super::*;

            #[test]
            fn empty_chain() {
                let data1: [u8; 0] = [];
                let data2: [u8; 0] = [];

                let mut buffer: Vec<u8> = Vec::new();

                let mut rev_chain = data1.as_slice().read_back_chain(data2.as_slice());

                assert_eq!(rev_chain.read_back(&mut buffer).ok(), Some(0));
                assert!(buffer.is_empty());
            }

            #[test]
            fn first_chain_full() {
                let data1: [u8; 3] = [1, 2, 3];
                let data2: [u8; 0] = [];

                let mut buffer: [u8; 4] = [0; 4];

                let mut rev_chain = data1.as_slice().read_back_chain(data2.as_slice());

                assert_eq!(rev_chain.read_back(&mut buffer).ok(), Some(3));
                assert_eq!(&buffer, &[0, 1, 2, 3]);
            }
        }

        mod read_back_bytes {
            use super::*;

            #[test]
            fn empty_data() {
                let data: [u8; 0] = [];

                let mut rev_bytes = data.as_slice().read_back_bytes();
                assert!(rev_bytes.next().is_none());
            }

            #[test]
            fn general() {
                let data: [u8; 3] = [1, 2, 3];

                let mut rev_bytes = data.as_slice().read_back_bytes();
                for byte_value in 3..=1 {
                    let next_value = rev_bytes.next();

                    assert!(&next_value.is_some());
                    assert!(next_value.as_ref().unwrap().is_ok());
                    assert_eq!(next_value.unwrap().unwrap(), byte_value);
                }
            }
        }
    }
}
