//! A crate for the reverse version of [std::io::Read].
//! You'll likely want to take a look at [rev_read::RevRead] and [rev_read::RevBufRead] to see which other types have
//! implemented it.
//!
//! # Example
//! `&[u8]` implements it for example:
//! ```
//! use rev_read::RevRead;
//! use std::io::Read;
//!
//! fn main() {
//!     let values = [1, 2, 3];
//!     let mut buffer = [0];
//!
//!     // How it could look like with `Read`:
//!     assert_eq!(values.as_slice().read(&mut buffer).ok(), Some(1));
//!     assert_eq!(buffer, [1]);
//!     println!("With Read: buffer = [{}]", buffer[0]);
//!
//!     // The reversed version:
//!     //                           [--] <- notice the `rev_` here
//!     assert_eq!(values.as_slice().rev_read(&mut buffer).ok(), Some(1));
//!     //                 [-] and the buffer contains the value starting from the back!
//!     assert_eq!(buffer, [3]);
//!     println!("With RevRead: buffer = [{}]", buffer[0]);
//! }
//! ```
mod impls;
mod rev_read;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
pub const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

pub use rev_read::{RevBufRead, RevBytes, RevChain, RevRead, RevSplit};
