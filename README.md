# Read Collection
This crate provides some other variants of the [`Read`] trait, like `ReadBack` or `RevRead`.

# Example (`ReadBack`)
```rust
use read_collection::ReadBack;
use std::io::Read;

fn main() {
    let values = [1, 2, 3];
    let mut buffer = [0];

    // How it could look like with `Read`:
    assert_eq!(values.as_slice().read(&mut buffer).ok(), Some(1));
    assert_eq!(buffer, [1]);

    // With `ReadBack`:
    assert_eq!(values.as_slice().read_back(&mut buffer).ok(), Some(1));
    //                 [-] and the buffer contains the value starting from the back!
    assert_eq!(buffer, [3]);
}
```

# Status
Implemented:
- [x] `ReadBack` for reading back *duh*
  - [x] `ReadBack` trait
    - [x] for `&[u8]`
    - [x] for [`File`] (and `&File`)
    - [x] for [`Empty`]
  - [x] `BufReadBack` trait
    - [x] for `&[u8]`
    - [x] for [`Empty`]
    - [x] `BufReadBacker` struct
 - [ ] `RevRead` for reading reversed
   - [ ] `RevRead` trait
     - [ ] for `&[u8]`
     - [ ] for [`File`] (and `&File`)
     - [ ] for [`Empty`]
   - [ ] `BufRevRead` trait
     - [ ] for `&[u8]`
     - [ ] for [`Empty`]
     - [ ] `BufRevReader` struct

[`File`]: https://doc.rust-lang.org/std/fs/struct.File.html
[`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
[`Empty`]: https://doc.rust-lang.org/std/io/struct.Empty.html
