use flate2::read::ZlibDecoder;
use std::io::Read;

pub fn flate_decode(bytes: &[u8]) -> Vec<u8> {
    let mut d = ZlibDecoder::new(bytes);
    // 10 times the size of the compressed stream --> improvement is required
    let buf = &mut vec![0; bytes.len() * 10];
    match d.read(buf) {
        Ok(_) => buf.to_vec(),
        Err(e) => {
            panic!("Error: {:?}", e);
        }
    }
}
