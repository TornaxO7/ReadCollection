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

// pub use bibufreader::BiBufReader;
pub use rev_borrowed_buf::RevBorrowedBuf;
pub use rev_buf_reader::RevBufRead;
pub use rev_reader::RevRead;
