use flate2::read::ZlibDecoder;
use std::io::Read;

pub fn flate_decode(bytes: &[u8]) -> String {
    println!("{:?}", bytes.len());
    let mut d = ZlibDecoder::new(bytes);
    // 10 times the size of the compressed stream
    let buf = &mut vec![0; bytes.len() * 10];
    match d.read(buf) {
        Ok(_) => (),
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }
    // sample.pdf contains non utf-8 characters in decompressed streams
    String::from_utf8_lossy(buf).to_string()
}
