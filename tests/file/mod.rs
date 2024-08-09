use std::fs::File;

mod buf_read_backer;
mod same_as_read;

fn get_file1() -> File {
    File::open("./tests/file/test_file1.txt").unwrap()
}

fn get_file2() -> File {
    File::open("./tests/file/test_file2.txt").unwrap()
}
