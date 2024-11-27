use crate::tokenizer::{Token, Tokenizer};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub struct ToUnicodeCMap {
    // page 472
    begin_code_space_range: Vec<u8>,
    end_code_space_range: Vec<u8>,
    pub is_two_bytes: bool, // size of char mapping index is 2 bytes (begincodespacerange)
    pub cmap: HashMap<usize, char>,
}

impl From<String> for ToUnicodeCMap {
    fn from(value: String) -> Self {
        let mut tokenizer = Tokenizer::new(value.as_bytes(), 0).peekable();

        // begincodespacerange - endcodespacerange -> size of mapping (1 or 2 bytes)
        for t in tokenizer.by_ref() {
            if let Token::String(s) = t {
                if s.to_vec() == b"begincodespacerange" {
                    break;
                }
            }
        }
        let begin_code_space_range = match tokenizer.next() {
            Some(Token::HexString(v)) => v,
            t => panic!(
                "Cmap begincodespacerange next token should be HexString, found {:?}",
                t
            ),
            None => panic!("Reached end of cmap stream before complete"),
        };
        let end_code_space_range = match tokenizer.next() {
            Some(Token::HexString(v)) => v,
            Some(t) => panic!(
                "Cmap endcodespacerange next token should be HexString, found {:?}",
                t
            ),
            None => panic!("Reached end of cmap stream before complete"),
        };
        let is_two_bytes = match begin_code_space_range.len() {
            1 => false,
            2 => true,
            n => panic!("Cmap index with byte length {n:?} is not supported"),
        };

        // Read CMap
        let mut cmap: HashMap<usize, char> = HashMap::new();
        while let Some(t) = tokenizer.next() {
            if let Token::String(s) = t {
                if s.to_vec() == b"beginbfchar" {
                    loop {
                        // end condition
                        if tokenizer.peek() == Some(&Token::String(b"endbfchar".to_vec())) {
                            break;
                        }
                        // BFChar mapping key (1 or 2 bytes depending on cmap codespacerange)
                        let key = match tokenizer.next() {
                            Some(Token::HexString(x)) => match x.len() {
                                2 => x[0] as usize * 256 + x[1] as usize,
                                1 => x[0] as usize,
                                n => {
                                    panic!("BFChar key should contain one or two bytes, found {n}")
                                }
                            },
                            Some(t) => panic!("CMap key should be an hex string, found {t:?}"),
                            None => panic!("CMap unreadable because end of cmap file is reached"),
                        };

                        // BFChar mapping value is a unicode character
                        let val = match tokenizer.next() {
                            Some(Token::HexString(x)) => {
                                let code = x[0] as u16 * 256 + x[1] as u16;
                                char::decode_utf16([code]).next().unwrap().ok().unwrap()
                            }
                            Some(t) => panic!("CMap val should be an hex string, found {t:?}"),
                            None => panic!("CMap unreadable because end of cmap file is reached"),
                        };
                        cmap.insert(key, val);
                    }
                }
                if s.to_vec() == b"beginbfrange" {
                    loop {
                        // end condition
                        if tokenizer.peek() == Some(&Token::String(b"endbfrange".to_vec())) {
                            break;
                        }
                        let src_code_1 = match tokenizer.next() {
                            Some(Token::HexString(x)) => match x.len() {
                                2 => x[0] as usize * 256 + x[1] as usize,
                                1 => x[0] as usize,
                                n => panic!("BFRange first source code should contain one or two bytes, found {n}")
                            },
                            Some(t) => panic!("CMap srcCode1 should be an hex string, found {t:?}"),
                            None => panic!("CMap unreadable because end of cmap file is reached"),
                        };
                        let src_code_2 = match tokenizer.next() {
                            Some(Token::HexString(x)) => match x.len() {
                                2 => x[0] as usize * 256 + x[1] as usize,
                                1 => x[0] as usize,
                                n => panic!("BFRange second source code should contain one or two bytes, found {n}")
                            },
                            Some(t) => panic!("CMap srcCode2 should be an hex string, found {t:?}"),
                            None => panic!("CMap unreadable because end of cmap file is reached"),
                        };
                        // BFRange destination strings
                        match tokenizer.next() {
                            // First unicode char
                            Some(Token::HexString(x)) => {
                                let mut dst_string = x[0] as u16 * 256 + x[1] as u16;
                                for idx in src_code_1..src_code_2 + 1 {
                                    cmap.insert(
                                        idx,
                                        char::decode_utf16([dst_string])
                                            .next()
                                            .unwrap()
                                            .ok()
                                            .unwrap(),
                                    );
                                    dst_string += 1;
                                }
                            }
                            // List of unicode chars
                            Some(Token::ArrayBegin) => {
                                let mut idx = 0;
                                loop {
                                    match tokenizer.next() {
                                    Some(Token::ArrayEnd) => break,
                                    Some(Token::HexString(x)) => {
                                        let code = x[0] as u16 * 256 + x[1] as u16;
                                        cmap.insert(src_code_1+idx, char::decode_utf16([code]).next().unwrap().ok().unwrap());
                                        idx += 1;
                                    },
                                    Some(t) => panic!("CMap range should only contain hex strings, found {t:?}"),
                                    None => panic!("CMap unreadable because end of cmap file is reached")
                                }
                                }
                            }
                            Some(t) => panic!(
                                "CMap dst_string should be an hex string or an array, found {t:?}"
                            ),
                            None => panic!("CMap unreadable because end of cmap file is reached"),
                        };
                    }
                }
            }
        }
        ToUnicodeCMap {
            begin_code_space_range,
            end_code_space_range,
            is_two_bytes,
            cmap,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_tounicode_cmap_0() {
        let cmap_string: String = String::from("/CIDInit/ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo<<\n/Registry (Adobe)\n/Ordering (UCS)\n/Supplement 0\n>> def\n/CMapName/Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n27 beginbfchar\n<01> <004C>\n<02> <006F>\n<03> <0072>\n<04> <0065>\n<05> <006D>\n<06> <0020>\n<07> <0069>\n<08> <0070>\n<09> <0073>\n<0A> <0075>\n<0B> <0064>\n<0C> <006C>\n<0D> <0074>\n<0E> <0061>\n<0F> <002C>\n<10> <0063>\n<11> <006E>\n<12> <0067>\n<13> <0079>\n<14> <0076>\n<15> <0062>\n<16> <0071>\n<17> <002E>\n<18> <0041>\n<19> <006A>\n<1A> <0053>\n<1B> <006B>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\nCMapNam");
        let tounicode: ToUnicodeCMap = ToUnicodeCMap::from(cmap_string);
        assert_eq!(tounicode.cmap.get(&1), Some(&'L'));
        assert_eq!(tounicode.cmap.get(&2), Some(&'o'));
        assert_eq!(tounicode.cmap.get(&3), Some(&'r'));
        assert_eq!(tounicode.cmap.get(&4), Some(&'e'));
        assert_eq!(tounicode.cmap.get(&5), Some(&'m'));
    }

    #[test]
    fn test_tounicode_cmap_1() {
        let cmap_string: String = String::from("/CIDInit /ProcSet findresource begin\n22 dict begin\nbegincmap\n/CIDSystemInfo\n<< /Registry (Adobe)\n/Ordering (UCS)\n/Supplement 0\n>> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n10 beginbfchar\n<0001> <0041>\n<001A> <0042>\n<001C> <0043>\n<0022> <0044>\n<0028> <0045>\n<003E> <0046>\n<0040> <0047>\n<0046> <0048>\n<0049> <0049>\n<005B> <004C>\nendbfchar\n1 beginbfrange\n<0061> <0062> <004D>\nendbfrange\n14 beginbfchar\n<0069> <004F>\n<0084> <0050>\n<0087> <0052>\n<008B> <0053>\n<0093> <0054>\n<0098> <0055>\n<00AB> <0056>\n<00AE> <0057>\n<00B4> <0059>\n<00CD> <0061>\n<00E6> <0062>\n<00E8> <0063>\n<00EE> <0064>\n<00F4> <0065>\nendbfchar\n1 beginbfrange\n<010B> <010C> <0066>\nendbfrange\n5 beginbfchar\n<0113> <0068>\n<0116> <0069>\n<0124> <006A>\n<0127> <006B>\n<012B> <006C>\nendbfchar\n1 beginbfrange\n<0131> <0132> <006D>\nendbfrange\n8 beginbfchar\n<013A> <006F>\n<0155> <0070>\n<0158> <0072>\n<015C> <0073>\n<0165> <0074>\n<016A> <0075>\n<017D> <0076>\n<017F> <0077>\nendbfchar\n1 beginbfrange\n<0184> <0185> <0078>\nendbfrange\n3 beginbfchar\n<018F> <007A>\n<01AF> <00660066>\n<01B1> <00660069>\nendbfchar\n1 beginbfrange\n<034F> <0358> <0030>\nendbfrange\n3 beginbfchar\n<03D9> <0020>\n<03DF> <002E>\n<03E2> <003B>\nendbfchar\n1 beginbfrange\n<03FC> <03FD> <0028>\nendbfrange\n1 beginbfchar\n<042D> <0026>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\nCMapNam");
        let tounicode: ToUnicodeCMap = ToUnicodeCMap::from(cmap_string);
        assert_eq!(tounicode.cmap.get(&1), Some(&'A'));
    }

    #[test]
    fn test_one_bfchar() {
        let cmap_string: String = String::from("begincodespacerange\n<0000> <FFFF>\nendcodespacerange\nbeginbfchar\n<03D9> <0020>\nendbfchar");
        let tounicode: ToUnicodeCMap = ToUnicodeCMap::from(cmap_string);
        assert_eq!(tounicode.cmap.get(&985), Some(&' '));
    }

    #[test]
    fn test_multiple_bfrange() {
        let cmap_string: String = String::from("begincodespacerange\n<0000> <FFFF>\nendcodespacerange\nbeginbfrange\n<03DF> <03E0> [<002E> <002C>]\n<03E1> <03E2> <003A>\nendbfrange");
        let tounicode: ToUnicodeCMap = ToUnicodeCMap::from(cmap_string);
        assert_eq!(tounicode.cmap.get(&991), Some(&'.'));
        assert_eq!(tounicode.cmap.get(&992), Some(&','));
        assert_eq!(tounicode.cmap.get(&993), Some(&':'));
    }
}
