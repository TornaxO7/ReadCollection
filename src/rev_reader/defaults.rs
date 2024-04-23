use std::io::{ErrorKind, IoSliceMut, Result};

use crate::{rev_borrowed_buf::RevBorrowedCursor, RevBorrowedBuf, RevRead, DEFAULT_BUF_SIZE};

pub(crate) fn default_rev_read_vectored<F>(
    rev_read: F,
    bufs: &mut [IoSliceMut<'_>],
) -> Result<usize>
where
    F: FnOnce(&mut [u8]) -> Result<usize>,
{
    let buf = bufs
        .iter_mut()
        .find(|b| !b.is_empty())
        .map_or(&mut [][..], |b| &mut **b);
    rev_read(buf)
}

pub(crate) fn default_rev_read_to_end<R: RevRead + ?Sized>(
    r: &mut R,
    buf: &mut Vec<u8>,
    size_hint: Option<usize>,
) -> Result<usize> {
    let start_len = buf.len();
    let start_cap = buf.capacity();
    // Optionally limit the maximum bytes read on each iteration.
    // This adds an arbitrary fiddle factor to allow for more data than we expect.
    let mut max_read_size = size_hint
        .and_then(|s| {
            s.checked_add(1024)?
                .checked_next_multiple_of(DEFAULT_BUF_SIZE)
        })
        .unwrap_or(DEFAULT_BUF_SIZE);

    let mut initialized = 0; // Extra initialized bytes from previous loop iteration

    const PROBE_SIZE: usize = 32;

    fn small_probe_read<R: RevRead + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> Result<usize> {
        let mut probe = [0u8; PROBE_SIZE];

        loop {
            match r.rev_read(&mut probe) {
                Ok(n) => {
                    // there is no way to recover from allocation failure here
                    // because the data has already been read.
                    buf.reverse();
                    buf.extend_from_slice(&probe[..n]);
                    buf.reverse();
                    return Ok(n);
                }
                Err(ref e) if ErrorKind::Interrupted == e.kind() => continue,
                Err(e) => return Err(e),
            }
        }
    }

    // avoid inflating empty/small vecs before we have determined that there's anything to read
    if (size_hint.is_none() || size_hint == Some(0)) && buf.capacity() - buf.len() < PROBE_SIZE {
        let read = small_probe_read(r, buf)?;

        if read == 0 {
            return Ok(0);
        }
    }

    loop {
        if buf.len() == buf.capacity() && buf.capacity() == start_cap {
            // The buffer might be an exact fit. Let's read into a probe buffer
            // and see if it returns `Ok(0)`. If so, we've avoided an
            // unnecessary doubling of the capacity. But if not, append the
            // probe buffer to the primary buffer and let its capacity grow.
            let read = small_probe_read(r, buf)?;

            if read == 0 {
                return Ok(buf.len() - start_len);
            }
        }

        if buf.len() == buf.capacity() {
            // buf is full, need more space
            buf.try_reserve(PROBE_SIZE)
                .map_err(|_| ErrorKind::OutOfMemory)?;
        }

        let mut spare = buf.spare_capacity_mut();
        let buf_len = std::cmp::min(spare.len(), max_read_size);
        spare = &mut spare[..buf_len];
        let mut read_buf: RevBorrowedBuf<'_> = spare.into();

        // SAFETY: These bytes were initialized but not filled in the previous loop
        unsafe {
            read_buf.set_init(initialized);
        }

        let mut cursor = read_buf.unfilled();
        loop {
            match r.rev_read_buf(cursor.reborrow()) {
                Ok(()) => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }

        let unfilled_but_initialized = cursor.init_ref().len();
        let bytes_read = cursor.written();
        let was_fully_initialized = read_buf.init_len() == buf_len;

        if bytes_read == 0 {
            return Ok(buf.len() - start_len);
        }

        // store how much was initialized but not filled
        initialized = unfilled_but_initialized;

        // SAFETY: BorrowedBuf's invariants mean this much memory is initialized.
        unsafe {
            let new_len = bytes_read + buf.len();
            buf.set_len(new_len);
        }

        // Use heuristics to determine the max read size if no initial size hint was provided
        if size_hint.is_none() {
            // The reader is returning short reads but it doesn't call ensure_init().
            // In that case we no longer need to restrict read sizes to avoid
            // initialization costs.
            if !was_fully_initialized {
                max_read_size = usize::MAX;
            }

            // we have passed a larger buffer than previously and the
            // reader still hasn't returned a short read
            if buf_len >= max_read_size && bytes_read == buf_len {
                max_read_size = max_read_size.saturating_mul(2);
            }
        }
    }
}

pub(crate) fn default_rev_read_to_string<R: RevRead + ?Sized>(
    r: &mut R,
    buf: &mut String,
    size_hint: Option<usize>,
) -> Result<usize> {
    todo!()
}

pub(crate) fn default_rev_read_exact<R: RevRead + ?Sized>(
    this: &mut R,
    mut buf: &mut [u8],
) -> Result<()> {
    while !buf.is_empty() {
        match this.rev_read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let buf_len = buf.len();
                buf = &mut buf[..buf_len - n];
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    if !buf.is_empty() {
        Err(std::io::Error::new(
            ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn default_rev_read_buf<F>(read: F, mut cursor: RevBorrowedCursor<'_>) -> Result<()>
where
    F: FnOnce(&mut [u8]) -> Result<usize>,
{
    let n = read(cursor.ensure_init().init_mut())?;
    unsafe {
        // SAFETY: we initialised using `ensure_init` so there is no uninit data to advance to.
        cursor.advance(n);
    }
    Ok(())
}

pub(crate) fn default_rev_read_buf_exact<R: RevRead + ?Sized>(
    read: &mut R,
    mut cursor: RevBorrowedCursor<'_>,
) -> Result<()> {
    while cursor.capacity() > 0 {
        let prev_written = cursor.written();
        match read.rev_read_buf(cursor.reborrow()) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }

        if cursor.written() == prev_written {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "failed to fill buffer",
            ));
        }
    }

    Ok(())
}
