# bgzf_rust_reader

This library helps to read the BGZF formatted file using RUST language. 

Extracted this information from: http://www.htslib.org/doc/bgzip.html

Bgzip compresses files in a similar manner to, and compatible with, gzip. 
The file is compressed into a series of small (less than 64K) 'BGZF' blocks. 
This allows indexes to be built against the compressed file and used to retrieve 
portions of the data without having to decompress the entire file. 

### Algorithm used: 
For decompresses 'deflate' algorithm is used, 
for more information please use this link: https://tools.ietf.org/html/rfc1951


## Usage
Below are the steps to use the bgzf Reader
1st step is to create a BGZF instance with a new function, after that read, seek etc method can be used.
```
use bgzf_rust_reader::BgzfReader;
use std::str;
let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
let mut vec = vec![0; 52];
let data_read = reader.read_to(&mut vec);
assert_eq!(data_read.unwrap(), 52);
assert_eq!("This is just a bgzf test,lets see how it reacts. :).", str::from_utf8(&vec).unwrap());
```

## Authors
Swoven Pokharel: swovenpokharel@gmail.com
