use crate::tokenizer::{Token, Tokenizer};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub struct ToUnicodeCMap(pub HashMap<usize, char>);

impl From<String> for ToUnicodeCMap {
    fn from(value: String) -> Self {
        let mut tokenizer = Tokenizer::new(value.as_bytes(), 0).peekable();
        for t in tokenizer.by_ref() {
            if let Token::String(s) = t {
                if s.to_vec() == b"beginbfchar" {
                    break;
                }
            }
        }
        let mut cmap = HashMap::new();
        loop {
            // end condition
            if tokenizer.peek() == Some(&Token::String(b"endbfchar".to_vec())) {
                break;
            }

            // key number in hex
            let key = match tokenizer.next() {
                Some(Token::HexString(x)) => x[0] as usize,
                Some(t) => panic!("CMap key should be an hex string, found {t:?}"),
                None => panic!("CMap unreadable because end of cmap file is reached"),
            };

            // unicode character encoded in hex
            let val = match tokenizer.next() {
                Some(Token::HexString(x)) => x[1] as char,
                Some(t) => panic!("CMap val should be an hex string, found {t:?}"),
                None => panic!("CMap unreadable because end of cmap file is reached"),
            };
            cmap.insert(key, val);
        }
        ToUnicodeCMap(cmap)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_tounicode_cmap() {
        let cmap_string: String = String::from("/CIDInit/ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo<<\n/Registry (Adobe)\n/Ordering (UCS)\n/Supplement 0\n>> def\n/CMapName/Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n27 beginbfchar\n<01> <004C>\n<02> <006F>\n<03> <0072>\n<04> <0065>\n<05> <006D>\n<06> <0020>\n<07> <0069>\n<08> <0070>\n<09> <0073>\n<0A> <0075>\n<0B> <0064>\n<0C> <006C>\n<0D> <0074>\n<0E> <0061>\n<0F> <002C>\n<10> <0063>\n<11> <006E>\n<12> <0067>\n<13> <0079>\n<14> <0076>\n<15> <0062>\n<16> <0071>\n<17> <002E>\n<18> <0041>\n<19> <006A>\n<1A> <0053>\n<1B> <006B>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\nCMapNam");
        let cmap: ToUnicodeCMap = ToUnicodeCMap::from(cmap_string);
        assert_eq!(cmap.0.get(&1), Some(&'L'));
        assert_eq!(cmap.0.get(&2), Some(&'o'));
        assert_eq!(cmap.0.get(&3), Some(&'r'));
        assert_eq!(cmap.0.get(&4), Some(&'e'));
        assert_eq!(cmap.0.get(&5), Some(&'m'));
    }
}
