use bgzf_rust_reader::BgzfReader;
use std::str;

#[test]
fn test_total_uncompressed_length() {
  let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
  let test_content = "This is just a bgzf test,lets see how it reacts. :). I think it will work fine, but who knows this is still a software. Unless you have tested it 100% there is no guarantee that it will work. So I am just trying to test bgzf with this text file. Have a great day software lovers. ";
  assert_eq!(
    reader.total_uncompressed_length(),
    test_content.len() as u64
  )
}

#[test]
fn test_random_access() {
  let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
  //In file bgzf_test (the uncompressed version) the 29th position in the file,
  //"This is just a bgzf test,lets" is upto this point
  reader.seek(29);
  let mut test_buffer = vec![0; 20];
  //reading 20 bytes to the vector test_buffer
  reader.read_to(&mut test_buffer);
  //the 20 bytes after 29th position in file is " see how it reacts. "
  assert_eq!(
    " see how it reacts. ",
    str::from_utf8(&test_buffer).unwrap()
  );
}
