use std::{fs::File, io::Read};

use rev_read::RevRead;

fn get_file() -> File {
    File::open("./tests/file/test_file.txt").unwrap()
}

#[test]
fn read_vs_rev_read() {
    let mut file = get_file();

    let mut read_buffer = [0u8; 5];
    let mut rev_read_buffer = read_buffer.clone();

    file.read(&mut read_buffer).unwrap();
    file.rev_read(&mut rev_read_buffer).unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
}

#[test]
fn read_to_end_vs_rev_read_to_end() {
    let mut file = get_file();

    let mut read_buffer = Vec::new();
    let mut rev_read_buffer = Vec::new();

    let read_amount = file.read_to_end(&mut read_buffer).unwrap();
    let rev_read_amount = file.rev_read_to_end(&mut rev_read_buffer).unwrap();

    assert_eq!(
        read_buffer,
        rev_read_buffer,
        "\n== Read ==\n{}\n== End-Read ==\n== RevRead ==\n{}\n== End-RevRead ==",
        String::from_utf8(read_buffer.clone()).unwrap(),
        String::from_utf8(rev_read_buffer.clone()).unwrap()
    );
    assert_eq!(read_amount, rev_read_amount);
}
