//! This crate provides a collection of different variations of [std::io::Read].
mod read_back;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

pub use read_back::{BufReadBack, ReadBack, ReadBackBytes, ReadBackChain, ReadBackSplit};
