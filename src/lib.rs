// #![feature(core_io_borrowed_buf)]
#![feature(new_uninit)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_write_slice)]
#![feature(read_buf)]
#![feature(ptr_as_uninit)]

mod impls;
mod rev_read;
mod rev_read_borrowed_buf;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
pub const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

pub use rev_read::{RevBufRead, RevRead};
pub use rev_read_borrowed_buf::{RevBorrowedBuf, RevBorrowedCursor};
