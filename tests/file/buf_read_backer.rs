use std::io::{BufReader, Read};

use read_collection::{BufReadBacker, ReadBack};

#[test]
fn buf_reader_vs_buf_read_backer() {
    let file = super::get_file1();

    let mut read_buffer = Vec::new();
    let mut read_back_buffer = Vec::new();

    let mut buf_reader = BufReader::new(file);
    let read_amount = buf_reader.read_to_end(&mut read_buffer).unwrap();

    let mut buf_read_backer = BufReadBacker::from(buf_reader);
    let read_back_amount = buf_read_backer
        .read_back_to_end(&mut read_back_buffer)
        .unwrap();

    assert_eq!(read_amount, read_back_amount);
    assert_eq!(read_buffer, read_back_buffer);
}
