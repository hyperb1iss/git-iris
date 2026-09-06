#![allow(clippy::unwrap_used)]

use std::io::Write;

use crate::agents::tools::file_read::{FileRead, FileReadArgs};

fn excerpt(content: &[u8], start: usize, count: usize) -> String {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    file.write_all(content).unwrap();
    FileRead::read_excerpt(
        file.path(),
        &FileReadArgs {
            path: "sample.txt".into(),
            start_line: Some(start),
            num_lines: Some(count),
        },
    )
    .unwrap()
}

#[test]
fn file_read_ranges_handle_eof_and_integer_limits() {
    for start in [3, 10, usize::MAX] {
        let output = excerpt(b"one\ntwo\n", start, 1000);
        assert!(output.contains("2 total lines"));
        assert!(output.contains("No lines in requested range"));
        assert!(!output.contains('│'));
    }
    assert!(excerpt(b"", 1, 500).contains("0 total lines"));
    assert!(!excerpt(b"one\n", 1, 0).contains('│'));
}

#[test]
fn file_read_excerpt_preserves_crlf_unicode_and_unterminated_lines() {
    let output = excerpt("one\r\n紫\r\nthree".as_bytes(), 2, 1);
    assert!(output.contains("3 total lines"));
    assert!(output.contains("     2│ 紫\n"));
    assert!(output.contains("start_line=3"));
    assert!(!output.contains("three"));
    assert!(excerpt(b"one\ntwo", 2, 2).contains("     2│ two\n"));
}

#[test]
fn file_read_skips_large_lines_without_retaining_them() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    let chunk = [b'x'; 8192];
    for _ in 0..2048 {
        file.write_all(&chunk).unwrap();
    }
    file.write_all(b"\nsmall excerpt\n").unwrap();
    let args = FileReadArgs {
        path: "generated.txt".into(),
        start_line: Some(2),
        num_lines: Some(1),
    };
    let output = FileRead::read_excerpt(file.path(), &args).unwrap();
    assert!(output.contains("2 total lines"));
    assert!(output.contains("2│ small excerpt"));
    assert!(output.len() < 200);
    let args = FileReadArgs {
        start_line: Some(1),
        ..args
    };
    assert!(
        FileRead::read_excerpt(file.path(), &args)
            .unwrap_err()
            .to_string()
            .contains("exceeds 10 MB")
    );
}

#[test]
fn file_read_detects_binary_content() {
    assert!(excerpt(b"hello\0world", 1, 10).contains("Binary file detected"));
    let mut content = vec![b'x'; 9000];
    content.extend_from_slice(b"\nhello\0world");
    assert!(excerpt(&content, 2, 1).contains("Binary file detected"));
}
