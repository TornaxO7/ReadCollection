mod impls;
mod rev_read;

// Bare metal platforms usually have very small amounts of RAM
// (in the order of hundreds of KB)
const DEFAULT_BUF_SIZE: usize = if cfg!(target_os = "espidf") {
    512
} else {
    8 * 1024
};

pub use rev_read::{BufReadBack, ReadBack, ReadBackBytes, ReadBackChain, RevSplit};
