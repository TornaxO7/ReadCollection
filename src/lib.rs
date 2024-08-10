//! This crate provides a collection of different variations of [std::io::Read].
//! At the moment, there's only [ReadBack]. Feel free to suggest other possible
//! [std::io::Read] variants which could be added to this collection.
//!
//! You'll likely want to use one of the following traits):
//! - [ReadBack]
//!
//! # Example with [ReadBack]
//! ```
//! use read_collection::ReadBack;
//! use std::io::Read;
//!
//! fn main() {
//!     let values = [1, 2, 3];
//!     let mut buffer = [0, 0];
//!
//!     // How it could look like with `Read`:
//!     assert_eq!(values.as_slice().read(&mut buffer).ok(), Some(2));
//!     assert_eq!(buffer, [1, 2]);
//!     println!("With Read: buffer = [{}, {}]", buffer[0], buffer[1]);
//!
//!     // The read-back version:
//!     assert_eq!(values.as_slice().read_back(&mut buffer).ok(), Some(2));
//!     //                 [-----] and the buffer contains the value starting from the back!
//!     assert_eq!(buffer, [2, 3]);
//!     println!("With ReadBack: buffer = [{}, {}]", buffer[0], buffer[1]);
//! }
//! ```
//! Output:
//! ```text
//! With Read: buffer = [1, 2]
//! With ReadBack: buffer = [2, 3]
//! ```
mod read_back;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

pub use read_back::{
    BufReadBack, BufReadBacker, ReadBack, ReadBackBytes, ReadBackChain, ReadBackSplit, ReadBackTake,
};
