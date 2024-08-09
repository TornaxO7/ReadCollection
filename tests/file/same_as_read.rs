use std::io::{Read, Seek};

use read_collection::ReadBack;

#[test]
fn read_vs_rev_read() {
    let mut file = super::get_file1();

    let mut read_buffer = [0u8; 5];
    let mut rev_read_buffer = read_buffer;

    file.read(&mut read_buffer).unwrap();
    file.read_back(&mut rev_read_buffer).unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
}

#[test]
fn read_to_end_vs_rev_read_to_end() {
    let mut file = super::get_file1();

    let mut read_buffer = Vec::new();
    let mut rev_read_buffer = Vec::new();

    let read_amount = file.read_to_end(&mut read_buffer).unwrap();
    let rev_read_amount = file.read_back_to_end(&mut rev_read_buffer).unwrap();

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
    let mut file = super::get_file1();

    let mut read_buffer = String::new();
    let mut rev_read_buffer = String::new();

    let read_amount = file.read_to_string(&mut read_buffer).unwrap();
    let rev_read_amount = file.read_back_to_string(&mut rev_read_buffer).unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
    assert_eq!(read_amount, rev_read_amount);
}

#[test]
fn read_exact_vs_rev_read_exact() {
    let mut file = super::get_file1();

    let mut read_buffer: [u8; 10] = [0; 10];
    let mut rev_read_buffer: [u8; 10] = [0; 10];

    file.read_exact(&mut read_buffer).unwrap();
    file.read_back_exact(&mut rev_read_buffer).unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
}

#[test]
fn read_bytes_vs_rev_read_bytes() {
    let file = super::get_file1();
    let mut file2 = super::get_file1();
    file2.seek(std::io::SeekFrom::End(0)).unwrap();

    let read_buffer = file.bytes().map(|b| b.unwrap()).collect::<Vec<u8>>();
    // todo!("Issue: If the cursor reached the start of the file => How do we differ between the first time we reach there and 'ok, we've read all bytes now'?");
    let mut rev_read_buffer = file2
        .read_back_bytes()
        .map(|b| b.unwrap())
        .collect::<Vec<u8>>();
    rev_read_buffer.reverse();

    assert_eq!(read_buffer, rev_read_buffer);
}

#[test]
fn read_chain_vs_rev_read_chain() {
    let read_file1 = super::get_file1();
    let read_file2 = super::get_file2();

    let mut rev_read_file1 = super::get_file1();
    let mut rev_read_file2 = super::get_file2();

    rev_read_file1.seek(std::io::SeekFrom::End(0)).unwrap();
    rev_read_file2.seek(std::io::SeekFrom::End(0)).unwrap();

    let mut read_chain = read_file1.chain(read_file2);
    let mut rev_read_chain = rev_read_file2.read_back_chain(rev_read_file1);

    let mut read_buffer = Vec::new();
    let mut rev_read_buffer = Vec::new();

    read_chain.read_to_end(&mut read_buffer).unwrap();
    rev_read_chain
        .read_back_to_end(&mut rev_read_buffer)
        .unwrap();

    assert_eq!(read_buffer, rev_read_buffer);
}
