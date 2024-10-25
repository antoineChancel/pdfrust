use std::io::Read;
use flate2::read::ZlibDecoder;

pub fn flate_decode(bytes: &[u8]) -> String {
    println!("{:?}", bytes.len());
    let mut d = ZlibDecoder::new(bytes);
    let mut s = String::new();
    match d.read_to_string(&mut s) {
        Ok(_) => (),
        Err(e) => {panic!("Error: {:?}", e);},
    }
    s
}
