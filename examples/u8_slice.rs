use read_collection::ReadBack;
use std::io::Read;

fn main() {
    let values = [1, 2, 3];
    let mut buffer = [0];

    // How it could look like with `Read`:
    assert_eq!(values.as_slice().read(&mut buffer).ok(), Some(1));
    assert_eq!(buffer, [1]);
    println!("With Read: buffer = [{}]", buffer[0]);

    // The readback version:
    //                           [--] <- notice the `rev_` here
    assert_eq!(values.as_slice().read_back(&mut buffer).ok(), Some(1));
    //                 [-] and the buffer contains the value starting from the back!
    assert_eq!(buffer, [3]);
    println!("With ReadBack: buffer = [{}]", buffer[0]);
}
