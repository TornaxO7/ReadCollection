use std::mem::MaybeUninit;

use crate::ReadBack;

/// Heavily inspired by the `std` implementation.
#[derive(Debug)]
pub struct Buffer {
    buf: Box<[MaybeUninit<u8>]>,
    pos: usize,
    filled: usize,
}

// methods which are similar to `BufReader`
impl Buffer {
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let buf = vec![MaybeUninit::uninit(); capacity].into_boxed_slice();
        Self {
            buf,
            pos: 0,
            filled: 0,
        }
    }

    #[inline]
    pub fn buffer(&self) -> &[u8] {
        // SAFETY: It's guaranteed that everything <= self.filled is initialised and self.pos <= self.filled
        unsafe { std::mem::transmute(&self.buf[self.pos..self.filled]) }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Returns the value of the `filled` position.
    #[inline]
    pub fn filled(&self) -> usize {
        self.filled
    }

    /// Returns the value of the `pos` position.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    pub fn discard_buffer(&mut self) {
        self.pos = 0;
        self.filled = 0;
    }

    #[inline]
    pub fn consume(&mut self, amt: usize) {
        self.pos = std::cmp::min(self.filled, self.pos + amt);
    }

    #[inline]
    pub fn fill_buf(&mut self, mut reader: impl ReadBack) -> std::io::Result<&[u8]> {
        // If we've reached the end of our internal buffer then we need to fetch
        // some more data from the reader.
        // Branch using `>=` instead of the more correct `==`
        // to tell the compiler that the pos..cap slice is always valid.
        if self.filled >= self.pos {
            debug_assert!(self.pos == self.filled);

            // SAFETY: It equals https://doc.rust-lang.org/src/core/mem/maybe_uninit.rs.html#995.
            let buffer =
                unsafe { &mut *(&mut self.buf[..] as *mut [MaybeUninit<u8>] as *mut [u8]) };

            self.pos = 0;
            self.filled = reader.read_back(buffer)?;
        }

        Ok(self.buffer())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_buffer() {
        let buffer = Buffer::with_capacity(10);

        assert!(buffer.buffer().is_empty());
        assert_eq!(buffer.pos, 0);
        assert_eq!(buffer.filled, 0);
    }

    #[test]
    fn partially_filled_buffer() {
        let data: &[u8] = &[1, 2, 3];
        let mut buffer = Buffer::with_capacity(5);

        assert_eq!(buffer.fill_buf(data).ok(), Some(data));
        assert_eq!(buffer.pos, 0);
        assert_eq!(buffer.filled, data.len());
    }
}
