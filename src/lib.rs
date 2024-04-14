#![feature(core_io_borrowed_buf)]
mod bibufreader;
mod rev_buf_reader;
mod rev_reader;

pub use bibufreader::BiBufReader;
pub use rev_buf_reader::RevBufRead;
pub use rev_reader::RevRead;
