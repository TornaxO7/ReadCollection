use std::{
    fs::File,
    io::{BorrowedBuf, Read, Seek},
};

use rev_read::{RevBorrowedBuf, RevRead};

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

#[test]
fn read_to_string_vs_rev_read_to_string() {
    let mut file = get_file();

    let mut read_buffer = String::new();
    let mut rev_read_buffer = String::new();

    let read_amount = file.read_to_string(&mut read_buffer).unwrap();
    let rev_read_amount = file.rev_read_to_string(&mut rev_read_buffer).unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
    assert_eq!(read_amount, rev_read_amount);
}

#[test]
fn read_exact_vs_rev_read_exact() {
    let mut file = get_file();

    let mut read_buffer: [u8; 10] = [0; 10];
    let mut rev_read_buffer: [u8; 10] = [0; 10];

    file.read_exact(&mut read_buffer).unwrap();
    file.rev_read_exact(&mut rev_read_buffer).unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
}

#[test]
fn read_buf_vs_rev_read_buf() {
    let mut file = get_file();

    let mut read_buffer: Vec<u8> = Vec::new();
    let mut rev_read_buffer: Vec<u8> = Vec::new();

    let mut borrowed_read_buffer = BorrowedBuf::from(read_buffer.as_mut_slice());
    let mut borrowed_rev_read_buffer = RevBorrowedBuf::from(rev_read_buffer.as_mut_slice());

    let cursor_read_buffer = borrowed_read_buffer.unfilled();
    let cursor_rev_read_buffer = borrowed_rev_read_buffer.unfilled();

    file.read_buf(cursor_read_buffer).unwrap();
    file.rev_read_buf(cursor_rev_read_buffer).unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
}

#[test]
fn read_bytes_vs_rev_read_bytes() {
    let file = get_file();
    let mut file2 = file.try_clone().unwrap();
    file2.seek(std::io::SeekFrom::End(0)).unwrap();

    let read_buffer = file.bytes().map(|b| b.unwrap()).collect::<Vec<u8>>();
    todo!("Issue: If the cursor reached the start of the file => How do we differ between the first time we reach there and 'ok, we've read all bytes now'?");
    let rev_read_buffer = file2.rev_bytes().map(|b| b.unwrap()).collect::<Vec<u8>>();

    assert_eq!(read_buffer, rev_read_buffer);
}
