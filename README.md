# bgzf_rust_reader

This library helps to read and provide Random Access to the BGZF(Bgzip) formatted file using RUST language.

Extracted from: http://www.htslib.org/doc/bgzip.html

"Bgzip compresses files in a similar manner to, and compatible with, gzip.
The file is compressed into a series of small (less than 64K) 'BGZF' blocks.
This allows indexes to be built against the compressed file and used to retrieve
portions of the data without having to decompress the entire file."

### Algorithm used:
For decompresses 'deflate' algorithm is used,
for more information please use this link: https://tools.ietf.org/html/rfc1951


## Usage
Below are the steps to use the bgzf Reader
1st step is to create a BGZF instance with a new function, after that read, seek etc method can be used for Random Access to the file.
```
use bgzf_rust_reader::BgzfReader;
use std::str;

//Getting the reader instance by using new function and passing the file path

let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();

//jumping to 29th position of the file starting from 0th index
reader.seek(29);

let mut test_buffer = vec![0; 20];

//reading 20 bytes to the vector test_buffer
reader.read_to(&mut test_buffer);

//the 20 bytes after 29th position in the example file is " see how it reacts. "
assert_eq!(
    " see how it reacts. ",
    str::from_utf8(&test_buffer).unwrap()
  );

```

## Authors
Swoven Pokharel: swovenpokharel@gmail.com
