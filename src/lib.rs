// #![feature(core_io_borrowed_buf)]
#![feature(new_uninit)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_write_slice)]
#![feature(read_buf)]
#![feature(ptr_as_uninit)]

// mod bibufreader;
mod rev_borrowed_buf;
mod rev_buf_reader;
mod rev_reader;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
pub const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

// pub use bibufreader::BiBufReader;
pub use rev_borrowed_buf::RevBorrowedBuf;
pub use rev_buf_reader::RevBufRead;
pub use rev_reader::RevRead;
