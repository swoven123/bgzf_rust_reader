use libdeflater::Decompressor;
use positioned_io::ReadAt;
use std::cell::Cell;
use std::cell::RefCell;
use std::cmp::min;
use std::collections::BTreeMap;
use std::error;
use std::fs::File;
use std::ops::Bound::{Excluded, Included};
use std::str;
use std::{error::Error, fmt};

/// Struct to hold the block information:
///
/// data_offset: pointer of file where real data is located,
/// data_length: total length of data i.e (block - header - footer,
/// input_length: uncompressed length of the data,
/// block_size: length of the block,
#[derive(Copy, Clone)]
struct BgzfBlock {
  data_offset: u64,
  data_length: u32,
  input_length: u32,
  block_size: u32,
}

///Cache struct to cache uncompressed data of a whole block
#[derive(Clone)]
struct Cache {
  pos: u64,
  uncompressed_data: Vec<u8>,
}

/// Struct to read bgzf file
///
/// Fields description:
///
/// input_length: total length of the uncompressed version,
/// current_read_position: current position of the compressed file,
/// pos: current position of the uncompressed file,
pub struct BgzfReader {
  bgzf_file: File,
  block_tree: BTreeMap<u64, BgzfBlock>,
  cache: RefCell<Option<Cache>>,
  pub input_length: u64,
  pub current_read_position: Cell<u64>,
  pub pos: Cell<u64>,
}

/// Below are the steps to use the bgzf Reader,
/// 1st step is to create a BGZF instance with a new function
/// after that read, and seek method can be used respectively.
///
/// # Example
/// ```
/// use bgzf_rust_reader::BgzfReader;
/// use std::str;
///
///  let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
///  let mut vec = vec![0; 52];
///  let data_read = reader.read_to(&mut vec);
///  assert_eq!(data_read.unwrap(), 52);
///  assert_eq!(
///    "This is just a bgzf test,lets see how it reacts. :).",
///    str::from_utf8(&vec).unwrap()
///  );
///
/// ```
impl BgzfReader {
  pub fn new(file_path: String) -> Result<BgzfReader, Box<dyn error::Error>> {
    let mut b_tree = BTreeMap::new();
    let bgzf_file = File::open(file_path)?;
    let mut input_offset: u64 = 0;
    let mut current_file_position = 0;
    loop {
      match read_block(&bgzf_file, current_file_position) {
        Ok(option_block) => match option_block {
          Some(block) => {
            let input_length_block = block.input_length;
            let block_size_block = block.block_size;
            b_tree.insert(input_offset, block);
            input_offset += u64::from(input_length_block);
            current_file_position += u64::from(block_size_block);
          }
          None => break,
        },
        Err(_e) => break,
      }
    }
    let reader = BgzfReader {
      bgzf_file,
      block_tree: b_tree,
      input_length: input_offset,
      current_read_position: Cell::new(0),
      pos: Cell::new(0),
      cache: RefCell::new(None),
    };
    Ok(reader)
  }

  /// This method can set the file position relative to uncompressed data
  ///
  /// # Example
  /// ```
  /// use bgzf_rust_reader::BgzfReader;
  ///
  ///let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
  ///reader.seek(33);
  /// assert_eq!(0, reader.current_read_position.get());
  ///assert_eq!(33, reader.pos.get());
  ///
  /// ```
  pub fn seek(&self, pos: u64) {
    self.pos.set(pos);
  }

  /// This method calculates total uncompressed length
  pub fn total_uncompressed_length(&self) -> u64 {
    self.input_length
  }

  /// this method reads data to the slice passed
  ///
  /// # Example
  /// ```
  /// use bgzf_rust_reader::BgzfReader;
  /// use std::str;
  ///
  ///  let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
  ///  let mut vec = vec![0; 52];
  ///  let data_read = reader.read_to(&mut vec);
  ///  assert_eq!(data_read.unwrap(), 52);
  ///  assert_eq!(
  ///    "This is just a bgzf test,lets see how it reacts. :).",
  ///    str::from_utf8(&vec).unwrap()
  ///  );
  ///
  /// ```
  pub fn read_to(&self, b: &mut Vec<u8>) -> Result<i32, Box<dyn error::Error>> {
    self.read(b, 0, b.len())
  }

  /// this method reads data to the slice from offset position,
  /// up to the len position
  ///
  /// # Example
  /// ```
  /// use bgzf_rust_reader::BgzfReader;
  /// use std::str;
  ///
  /// let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
  /// let mut content = vec![0; 10];
  /// match reader.read(&mut content, 0, 10) {
  ///  Ok(val) => {
  ///   assert_eq!(10, val);
  ///  }
  ///  Err(e) => {
  ///    assert!(false);
  ///  }
  /// };
  ///let file_content = str::from_utf8(&content).unwrap();
  ///  assert_eq!("This is ju", file_content);
  ///
  /// ```
  pub fn read(
    &self,
    b: &mut Vec<u8>,
    off: usize,
    len: usize,
  ) -> Result<i32, Box<dyn error::Error>> {
    if b.len() == 0 {
      return Err(BGZFError::new("Buffer size needs to be greater than 0").into());
    }
    if len > b.len() - off {
      return Err(BGZFError::new("Index out of bound exception").into());
    }
    if len == 0 {
      return Ok(0);
    }
    if self.pos.get() >= self.input_length {
      return Ok(-1);
    }

    let mut off = off;
    let mut len = len;
    let mut cb: i32 = 0;

    match self.cache.borrow().as_ref() {
      Some(cache) => {
        if self.pos.get() >= cache.pos {
          let bytes_available_in_cache =
            cache.pos as usize + cache.uncompressed_data.len() - self.pos.get() as usize;
          if bytes_available_in_cache > 0 {
            let copy_start = (self.pos.get() - cache.pos) as usize;
            let copy_length = min(bytes_available_in_cache, len);
            let end_index = copy_start + copy_length;
            b[off..]
              .copy_from_slice(&cache.uncompressed_data[copy_start as usize..end_index as usize]);
            cb += copy_length as i32;
            off += copy_length;
            len -= copy_length;
            self.pos.set(self.pos.get() + copy_length as u64);
            if len == 0 {
              return Ok(cb);
            }
          }
        }
      }
      None => {
        //If there is no cache available lets move forward
      }
    }

    let mut un_compressor = Decompressor::new();

    #[derive(Copy, Clone)]
    struct Entry {
      key: u64,
      value: BgzfBlock,
    }

    let mut entry_vector: Vec<Entry> = Vec::new();

    if !self.block_tree.contains_key(&self.pos.get()) {
      let floored_value = self.block_tree.range(..self.pos.get()).next_back().unwrap();
      //Getting a floored value if we do not find pos in the tree.
      entry_vector.push(Entry {
        key: *floored_value.0,
        value: *floored_value.1,
      });
    }
    //Get all the blocks from the block tree that is within the range of
    //pos and length of the buffer passed
    let pos_and_len_combined = self.pos.get() + len as u64;
    for (&key, &value) in self
      .block_tree
      .range((Included(self.pos.get()), Excluded(pos_and_len_combined)))
    {
      entry_vector.push(Entry { key, value });
    }

    for entry in entry_vector {
      let block = entry.value;
      let input_offset = entry.key;

      //Reading compressed data from the block
      let mut compressed = vec![0u8; block.data_length as usize];
      self
        .bgzf_file
        .read_exact_at(block.data_offset, &mut compressed)?;

      //now it's time to de-compress the read value obtained.
      let mut uncompressed = vec![0u8; block.input_length as usize];
      let bytes_decompressed =
        un_compressor.deflate_decompress(&mut compressed, &mut uncompressed)?;

      if bytes_decompressed == 0 || bytes_decompressed != block.input_length as usize {
        return Err(BGZFError::new("Did not fully de-compress").into());
      }

      self.cache.replace(Some(Cache {
        pos: input_offset,
        uncompressed_data: uncompressed.clone(),
      }));

      let mut copy_start: u64 = 0;
      //total uncompressed size is input_length
      let mut copy_length = block.input_length;
      if input_offset < self.pos.get() {
        let copy_skip = self.pos.get() - input_offset;
        copy_start += copy_skip;
        copy_length -= copy_skip as u32;
      }

      if copy_length > len as u32 {
        copy_length = len as u32;
      }
      let end_index = copy_start + u64::from(copy_length);
      b[off..].copy_from_slice(&uncompressed[copy_start as usize..end_index as usize]);
      len -= copy_length as usize;
      self.pos.set(self.pos.get() + u64::from(copy_length));
      off += copy_length as usize;
      cb += copy_length as i32;
    }
    Ok(cb)
  }
}

fn read_block(
  file: &File,
  current_file_position: u64,
) -> Result<Option<BgzfBlock>, Box<dyn error::Error>> {
  let mut current_file_position = current_file_position;

  let mut buf = [0; 12];
  file.read_exact_at(current_file_position, &mut buf)?;
  current_file_position += buf.len() as u64;

  if buf[0] != 31 || buf[1] != 139 || buf[2] != 8 || buf[3] != 4 {
    return Err(BGZFError::new("Incorrect header").into());
  }

  let xlen: u16 = (buf[10] as u16) | ((buf[11] as u16) << 8);

  let mut buf_xlen = vec![0u8; usize::from(xlen)];

  file.read_exact_at(current_file_position, &mut buf_xlen)?;
  current_file_position += buf_xlen.len() as u64;

  if buf_xlen[0] != 66 || buf_xlen[1] != 67 {
    return Err(BGZFError::new("Bad subfield Identifier").into());
  }

  if ((buf_xlen[2] as u16) | ((buf_xlen[3] as u16) << 8)) != 2 {
    return Err(BGZFError::new("Bad subfield Length").into());
  }

  let bsize = (buf_xlen[4] as u16) | ((buf_xlen[5] as u16) << 8);
  let block_size = u32::from(bsize) + 1;
  let data_length = bsize - xlen - 19;
  let data_offset = current_file_position;

  //Skip data block
  current_file_position += u64::from(data_length) + 4;

  let mut buf_isize = [0; 4];
  file.read_exact_at(current_file_position, &mut buf_isize)?;

  let i_size: u32 = (buf_isize[0] as u32)
    | ((buf_isize[1] as u32) << 8)
    | ((buf_isize[2] as u32) << 16)
    | ((buf_isize[3] as u32) << 24);

  if i_size == 0 {
    return Ok(None);
  }

  let block = BgzfBlock {
    data_offset,
    data_length: u32::from(data_length),
    input_length: u32::from(i_size),
    block_size,
  };
  Ok(Some(block))
}

#[derive(Debug)]
struct BGZFError {
  msg: String,
}

impl BGZFError {
  fn new(msg: &str) -> BGZFError {
    BGZFError {
      msg: msg.to_string(),
    }
  }
}

impl Error for BGZFError {
  fn description(&self) -> &str {
    &self.msg
  }
}

impl fmt::Display for BGZFError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.msg)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_read_block_func() {
    let bgzf_file = File::open("bgzf_test.bgz").unwrap();
    match read_block(&bgzf_file, 0) {
      Ok(option_block) => match option_block {
        Some(block) => {
          assert_eq!(block.block_size, 211);
          assert_eq!(block.data_length, 185);
          assert_eq!(block.data_offset, 18);
          assert_eq!(block.input_length, 280);
        }
        None => assert!(false),
      },
      Err(_e) => assert!(false),
    }
  }

  #[test]
  fn test_bgzf_reader_new_func() {
    let bgzf_reader = BgzfReader::new(String::from("bgzf_test.bgz"));
    match bgzf_reader {
      Ok(reader) => {
        let expected_uncompressed_length = 280;
        assert_eq!(1, reader.block_tree.len());
        assert_eq!(expected_uncompressed_length, reader.input_length);
        assert_eq!(0, reader.current_read_position.get());

        let block = reader.block_tree.get(&0);
        match block {
          Some(block) => {
            assert_eq!(block.block_size, 211);
            assert_eq!(block.data_length, 185);
            assert_eq!(block.data_offset, 18);
            assert_eq!(block.input_length, 280);
          }
          None => assert!(false),
        }
      }
      Err(_e) => assert!(false),
    }
  }

  #[test]
  fn test_bgzf_read_method() {
    let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
    let mut content = vec![0; 10];
    match reader.read(&mut content, 0, 10) {
      Ok(val) => {
        assert_eq!(10, val);
      }
      Err(e) => {
        assert!(false);
      }
    };
    let file_content = str::from_utf8(&content).unwrap();
    assert_eq!("This is ju", file_content);

    reader.seek(20);
    let mut content_two = vec![0; 32];
    match reader.read(&mut content_two, 0, 32) {
      Ok(val) => {
        assert_eq!(32, val);
      }
      Err(_e) => {
        assert!(false);
      }
    };
    let file_content_two = str::from_utf8(&content_two).unwrap();
    assert_eq!("test,lets see how it reacts. :).", file_content_two);
  }

  #[test]
  fn test_seek_method() {
    let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
    reader.seek(33);
    assert_eq!(0, reader.current_read_position.get());
    assert_eq!(33, reader.pos.get());
  }

  #[test]
  fn test_read_to() {
    let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
    let mut vec = vec![0; 52];
    let data_read = reader.read_to(&mut vec);
    assert_eq!(data_read.unwrap(), 52);
    assert_eq!(
      "This is just a bgzf test,lets see how it reacts. :).",
      str::from_utf8(&vec).unwrap()
    );
  }

  #[test]
  fn test_cache() {
    let reader = BgzfReader::new(String::from("bgzf_test.bgz")).unwrap();
    let mut vec = vec![0; 52];
    let data_read = reader.read_to(&mut vec);
    assert_eq!(data_read.unwrap(), 52);
    assert_eq!(
      "This is just a bgzf test,lets see how it reacts. :).",
      str::from_utf8(&vec).unwrap()
    );

    let mut vec2 = vec![0; 119];
    let data_read_2 = reader.read_to(&mut vec2);
    assert_eq!(data_read_2.unwrap(), 119);
    assert_eq!(
    " I think it will work fine, but who knows this is still a software. Unless you have tested it 100% there is no guarante",
      str::from_utf8(&vec2).unwrap()
    );

    let mut vec3 = vec![0; 2];
    let data_read_3 = reader.read_to(&mut vec3);
    assert_eq!(data_read_3.unwrap(), 2);
    assert_eq!("e ", str::from_utf8(&vec3).unwrap());
  }

}
