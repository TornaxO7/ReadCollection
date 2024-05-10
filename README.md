# RevRead
This crate provides the reversed version of the [`Read`] trait with some implementors where it's suitable.
You just need to add the prefix `rev_` to all functions of [`Read`] and you get its reversed version.

# Example
```rust
use std::io::Read;
use rev_read::RevRead;

fn main() {
  let values = [1, 2, 3];
  let mut buffer = [0];

  // How it could look like with `Read`:
  assert_eq!(values.as_slice().read(&mut buffer).ok(), Some(1));
  assert_eq!(buffer, [1]);

  // The reversed version:
  //                           [--] <- notice the `rev_` here
  assert_eq!(values.as_slice().rev_read(&mut buffer).ok(), Some(1));
  //                 [-] and the buffer contains the value starting from the back!
  assert_eq!(buffer, [3]);
}
```

[`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
