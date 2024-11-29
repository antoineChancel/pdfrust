use core::panic;
use std::{char, iter::Peekable, slice::Iter};

use crate::{algebra::Number, xref::XrefTable};

// Tokenizer for PDF objects
#[derive(Debug)]
pub enum WhiteSpace {
    Null,
    Tab,
    LineFeed,
    FormFeed,
    CarriageReturn,
    Space,
}

impl WhiteSpace {
    fn new(char: u8) -> WhiteSpace {
        match char {
            0 => WhiteSpace::Null,
            9 => WhiteSpace::Tab,
            10 => WhiteSpace::LineFeed,
            12 => WhiteSpace::FormFeed,
            13 => WhiteSpace::CarriageReturn,
            32 => WhiteSpace::Space,
            _ => panic!("Unable to interprete character set whitespace"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Token<'a> {
    Numeric(Number),
    String(Vec<u8>),
    LitteralString(Vec<u8>),
    HexString(Vec<u8>),
    Name(String),
    Comment(Vec<u8>),
    IndirectRef((i16, i16), &'a XrefTable, &'a [u8]),
    DictBegin,
    DictEnd,
    ArrayBegin,
    ArrayEnd,
    StreamBegin,
    StreamEnd,
    ObjBegin,
    ObjEnd,
}

#[derive(Debug)]
pub enum Delimiter {
    String,
    Array,
    Name,
    Comment,
}

impl Delimiter {
    fn new(char: u8) -> Delimiter {
        match char {
            b'(' | b')' => Delimiter::String,
            b'<' | b'>' | b'[' | b']' | b'{' | b'}' => Delimiter::Array,
            b'/' => Delimiter::Name,
            b'%' => Delimiter::Comment,
            _ => panic!("Unable to interprete character set delimiter"),
        }
    }
}

#[derive(Debug)]
pub enum CharacterSet {
    Regular(u8),
    Delimiter(Delimiter),
    WhiteSpace(WhiteSpace),
}

impl From<&u8> for CharacterSet {
    fn from(char: &u8) -> CharacterSet {
        match char {
            0 | 9 | 10 | 12 | 13 | 32 => CharacterSet::WhiteSpace(WhiteSpace::new(*char)),
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' => {
                CharacterSet::Delimiter(Delimiter::new(*char))
            }
            _ => CharacterSet::Regular(*char),
        }
    }
}

pub struct Lemmatizer<'a> {
    tokenizer: Tokenizer<'a>,
    xref: &'a XrefTable,
}

impl<'a> Lemmatizer<'a> {
    pub fn new(bytes: &'a [u8], curr_idx: usize, xref: &'a XrefTable) -> Lemmatizer<'a> {
        Lemmatizer {
            tokenizer: Tokenizer::new(bytes, curr_idx),
            xref,
        }
    }

    pub fn next_n(&mut self, length: usize) -> Vec<u8> {
        self.tokenizer.next_n(length)
    }
}

impl<'a> Iterator for Lemmatizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.tokenizer.next() {
            Some(Token::Numeric(Number::Integer(a))) => {
                // try reading a indirect reference or object reference
                let mut new_tokenizer = self.tokenizer.clone();
                match new_tokenizer.next() {
                    Some(Token::Numeric(Number::Integer(b))) => match new_tokenizer.next() {
                        Some(Token::String(s)) => match s.as_slice() {
                            b"R" => {
                                self.tokenizer.next();
                                self.tokenizer.next();
                                return Some(Token::IndirectRef(
                                    (a, b),
                                    self.xref,
                                    self.tokenizer.bytes,
                                ));
                            }
                            b"obj" => {
                                self.tokenizer.next();
                                self.tokenizer.next();
                                return Some(Token::ObjBegin);
                            }
                            _ => (),
                        },
                        _ => return Some(Token::Numeric(Number::Integer(a))),
                    },
                    _ => return Some(Token::Numeric(Number::Integer(a))),
                }
            }
            Some(Token::Comment(_)) => return self.next(), // skip to next token
            Some(t) => return Some(t),
            None => return None,
        };
        None
    }
}

#[derive(Clone)]
pub struct Tokenizer<'a> {
    bytes: &'a [u8],
    byte: Peekable<Iter<'a, u8>>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(bytes: &'a [u8], curr_idx: usize) -> Tokenizer<'a> {
        Tokenizer {
            bytes,
            byte: bytes[curr_idx..].iter().peekable(),
        }
    }

    pub fn next_n(&mut self, length: usize) -> Vec<u8> {
        // skip whitespaces characters
        loop {
            match self.byte.peek() {
                Some(&a) => match CharacterSet::from(a) {
                    CharacterSet::WhiteSpace(_) => self.byte.next(),
                    _ => break,
                },
                None => panic!("End of stream reached"),
            };
        }
        self.byte.clone().take(length).copied().collect::<Vec<u8>>()
    }

    fn byte_to_digit(b: &u8) -> u8 {
        match b {
            b'0' => 0,
            b'1' => 1,
            b'2' => 2,
            b'3' => 3,
            b'4' => 4,
            b'5' => 5,
            b'6' => 6,
            b'7' => 7,
            b'8' => 8,
            b'9' => 9,
            _ => panic!(),
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(c) = self.byte.next() {
            match CharacterSet::from(c) {
                CharacterSet::Delimiter(v) => match v {
                    Delimiter::Comment => {
                        // read all characters until a line feed or cariage return is met
                        let mut buf: Vec<u8> = vec![];
                        while let Some(c) = self.byte.peek() {
                            // end of stream
                            match CharacterSet::from(*c) {
                                CharacterSet::WhiteSpace(WhiteSpace::CarriageReturn) => break,
                                CharacterSet::WhiteSpace(WhiteSpace::LineFeed) => break,
                                _ => buf.push(**c),
                            }
                            self.byte.next();
                        }
                        return Some(Token::Comment(buf));
                    }
                    Delimiter::Array => {
                        match c {
                            b'<' => match self.byte.peek() {
                                // begin dictionary
                                Some(b'<') => {
                                    self.byte.next();
                                    return Some(Token::DictBegin);
                                }
                                // Hexadecimal characters
                                Some(
                                    b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8'
                                    | b'9' | b'A' | b'B' | b'C' | b'D' | b'E' | b'F' | b'a' | b'b'
                                    | b'c' | b'd' | b'e' | b'f',
                                ) => {
                                    let mut buf: String = String::new();
                                    loop {
                                        match self.byte.next() {
                                            Some(b'>') => break,
                                            Some(a) => buf.push(*a as char),
                                            None => return None
                                        }
                                    }
                                    // HexString should contain a pair number of characters to be valid
                                    if buf.len() % 2 == 1 {
                                        buf.push('0');
                                    }
                                    // println!("{:?}", &buf);
                                    // Decode hex to u8
                                    let buf_decoded: Vec<u8> = (0..buf.len())
                                        .step_by(2)
                                        .map(|i| u8::from_str_radix(&buf[i..i + 2], 16).unwrap())
                                        .collect();
                                    return Some(Token::HexString(buf_decoded));
                                }
                                Some(b) => panic!("Character {b:} is not interpreted as hexstring"),
                                None => panic!("No character following '<'"),
                            },
                            b'>' => match self.byte.peek() {
                                Some(b'>') => {
                                    self.byte.next();
                                    return Some(Token::DictEnd);
                                }
                                Some(_) => continue,
                                None => panic!("Reached end of stream before end of litteral"),
                            },
                            b'[' => return Some(Token::ArrayBegin),
                            b']' => return Some(Token::ArrayEnd),
                            l => panic!("Character {l} is not covered"),
                        }
                    }
                    Delimiter::Name => {
                        let mut buf: String = String::new();
                        while let Some(a) = self.byte.peek() {
                            match CharacterSet::from(*a) {
                                CharacterSet::Regular(a) => buf.push(a as char),
                                _ => break,
                            }
                            self.byte.next();
                        }
                        return Some(Token::Name(buf));
                    }
                    // litteral string
                    Delimiter::String => {
                        let mut buf: Vec<u8> = vec![];
                        // nested parentesis counters
                        let mut opened_parathesis: u8 = 1;
                        let mut closed_parathesis: u8 = 0;
                        while let Some(cursor) = self.byte.next() {
                            match cursor {
                                b'(' => opened_parathesis += 1,
                                b')' => closed_parathesis += 1,
                                _ => (),
                            };
                            if opened_parathesis == closed_parathesis {
                                break;
                            };
                            let c = match cursor {
                                // table 3.2, page 54
                                b'\\' => match self.byte.next() {
                                    Some(c) => match c {
                                        b'n' => b'\n',
                                        b'r' => b'\r',
                                        b't' => b'\t',
                                        b'b' => 8,
                                        b'f' => 12,
                                        b'\\' => b'\\',
                                        b'(' => b'(',
                                        b')' => b')',
                                        b'0'..=b'9' => {
                                            // convert octal digit to u8
                                            let c = Tokenizer::byte_to_digit(c);
                                            let d =
                                                Tokenizer::byte_to_digit(self.byte.next().unwrap());
                                            let e =
                                                Tokenizer::byte_to_digit(self.byte.next().unwrap());
                                            ((c * 8) + d * 8) + e
                                        }
                                        c => *c,
                                    },
                                    None => continue,
                                },
                                b => *b,
                            };
                            buf.push(c)
                        }
                        return Some(Token::LitteralString(buf));
                    }
                },
                // read regular string
                CharacterSet::Regular(_) => {
                    let mut buf: Vec<u8> = vec![];
                    buf.push(*c);
                    let mut is_numeric = true;
                    while let Some(&c) = self.byte.peek() {
                        match CharacterSet::from(c) {
                            CharacterSet::Regular(
                                b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
                                | b'.',
                            ) => (),
                            CharacterSet::Regular(_) => is_numeric = false,
                            _ => break,
                        }
                        buf.push(*c);
                        self.byte.next();
                    }
                    if is_numeric {
                        let numeric = std::str::from_utf8(&buf).unwrap();
                        match numeric.parse::<i16>() {
                            Ok(n) => return Some(Token::Numeric(Number::Integer(n))),
                            Err(_) => {
                                if let Ok(n) = numeric.parse::<f32>() {
                                    return Some(Token::Numeric(Number::Real(n)));
                                }
                            }
                        }
                    };
                    match buf.as_slice() {
                        b"stream" => return Some(Token::StreamBegin),
                        b"endstream" => return Some(Token::StreamEnd),
                        b"endobj" => return Some(Token::ObjEnd),
                        _ => return Some(Token::String(buf)),
                    }
                }
                // absorb whitespaces before a new token is met
                CharacterSet::WhiteSpace(_) => continue,
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_pdfbytes_iterator_skipped_comment() {
        let mut pdf = Tokenizer::new(b"%PDF-1.7\n\n1 0 obj  % entry point", 0);
        assert_eq!(pdf.next(), Some(Token::Comment(b"PDF-1.7".to_vec())));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(1))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::String(b"obj".to_vec())));
        assert_eq!(pdf.next(), Some(Token::Comment(b" entry point".to_vec())));
        assert_eq!(pdf.next(), None);
    }

    #[test]
    fn test_litteral_string_octal() {
        let mut pdf = Tokenizer::new(b"(\\003)", 0);
        assert_eq!(pdf.next(), Some(Token::LitteralString(vec![3])))
    }

    #[test]
    fn test_pdfbytes_iterator_litteral_string() {
        let mut pdf = Tokenizer::new(b"(Hello World)", 0);
        assert_eq!(
            pdf.next(),
            Some(Token::LitteralString(b"Hello World".to_vec()))
        );
    }

    #[test]
    fn test_pdfbytes_iterator_litteral_string_with_embedded_parenthesis() {
        let mut pdf = Tokenizer::new(b"((Hello) (World))", 0);
        assert_eq!(
            pdf.next(),
            Some(Token::LitteralString(b"(Hello) (World)".to_vec()))
        );
    }

    #[test]
    fn test_pdfbytes_iterator_hex_string() {
        let mut pdf = Tokenizer::new(b"<4E6F762073686D6F7A206B6120706F702E>", 0);
        assert_eq!(
            pdf.next(),
            Some(Token::HexString(
                [78, 111, 118, 32, 115, 104, 109, 111, 122, 32, 107, 97, 32, 112, 111, 112, 46]
                    .to_vec()
            ))
        );
    }

    #[test]
    fn test_pdfbytes_numeric_float() {
        let mut pdf = Tokenizer::new(b"12.34", 0);
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Real(12.34))));
    }

    #[test]
    fn test_pdfbytes_mediabox_float() {
        let mut pdf = Tokenizer::new(b"/MediaBox [ 0 0 200.00 200.00 ] ", 0);
        assert_eq!(pdf.next(), Some(Token::Name("MediaBox".to_string())));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Real(200.0))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Real(200.0))));
        assert_eq!(pdf.next(), Some(Token::ArrayEnd));
    }

    #[test]
    fn test_tokenizer_1() {
        let mut pdf = Tokenizer::new(b"2 0 obj\n<<\n  /Type /Pages\n  /MediaBox [ 0 0 200 200 ]\n  /Count 1\n  /Kids [ 3 0 R ]\n>>\nendobj\n", 0);
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(2))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::String(b"obj".to_vec())));
        assert_eq!(pdf.next(), Some(Token::DictBegin));
        assert_eq!(pdf.next(), Some(Token::Name("Type".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("Pages".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("MediaBox".to_string())));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(200))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(200))));
        assert_eq!(pdf.next(), Some(Token::ArrayEnd));
        assert_eq!(pdf.next(), Some(Token::Name("Count".to_string())));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(1))));
        assert_eq!(pdf.next(), Some(Token::Name("Kids".to_string())));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(3))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::String(b"R".to_vec())));
        assert_eq!(pdf.next(), Some(Token::ArrayEnd));
        assert_eq!(pdf.next(), Some(Token::DictEnd));
        assert_eq!(pdf.next(), Some(Token::ObjEnd));
    }

    #[test]
    fn test_tokenizer() {
        let mut pdf = Tokenizer::new(b"9 0 obj\n<</Type/Font/Subtype/TrueType/BaseFont/BAAAAA+DejaVuSans\n/FirstChar 0\n/LastChar 27\n/Widths[600 557 611 411 615 974 317 277 634 520 633 634 277 392 612 317\n549 633 634 591 591 634 634 317 684 277 634 579 ]\n/FontDescriptor 7 0 R\n/ToUnicode 8 0 R\n>>", 0);
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(9))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::String(b"obj".to_vec())));
        assert_eq!(pdf.next(), Some(Token::DictBegin));
        assert_eq!(pdf.next(), Some(Token::Name("Type".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("Font".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("Subtype".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("TrueType".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("BaseFont".to_string())));
        assert_eq!(
            pdf.next(),
            Some(Token::Name("BAAAAA+DejaVuSans".to_string()))
        );
        assert_eq!(pdf.next(), Some(Token::Name("FirstChar".to_string())));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Name("LastChar".to_string())));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(27))));
        assert_eq!(pdf.next(), Some(Token::Name("Widths".to_string())));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(600))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(557))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(611))));
    }

    #[test]
    fn test_lemmatizer_1() {
        let xref = XrefTable::new();
        let mut pdf = Lemmatizer::new(b"9 0 obj\n<</Type/Font/Subtype/TrueType/BaseFont/BAAAAA+DejaVuSans\n/FirstChar 0\n/LastChar 27\n/Widths[600 557 611 411 615 974 317 277 634 520 633 634 277 392 612 317\n549 633 634 591 591 634 634 317 684 277 634 579 ]\n/FontDescriptor 7 0 R\n/ToUnicode 8 0 R\n>>", 0, &xref);
        assert_eq!(pdf.next(), Some(Token::ObjBegin));
        assert_eq!(pdf.next(), Some(Token::DictBegin));
        assert_eq!(pdf.next(), Some(Token::Name("Type".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("Font".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("Subtype".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("TrueType".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("BaseFont".to_string())));
        assert_eq!(
            pdf.next(),
            Some(Token::Name("BAAAAA+DejaVuSans".to_string()))
        );
        assert_eq!(pdf.next(), Some(Token::Name("FirstChar".to_string())));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Name("LastChar".to_string())));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(27))));
        assert_eq!(pdf.next(), Some(Token::Name("Widths".to_string())));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(600))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(557))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(611))));
    }

    #[test]
    fn test_lemmatizer_0() {
        let xref = XrefTable::new();
        let bytes = b"2 0 obj\n<<\n  /Type /Pages\n  /MediaBox [ 0 0 200 200 ]\n  /Count 1\n  /Kids [ 3 0 R ]\n>>\nendobj\n";
        let mut pdf = Lemmatizer::new(bytes, 0, &xref);
        assert_eq!(pdf.next(), Some(Token::ObjBegin));
        assert_eq!(pdf.next(), Some(Token::DictBegin));
        assert_eq!(pdf.next(), Some(Token::Name("Type".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("Pages".to_string())));
        assert_eq!(pdf.next(), Some(Token::Name("MediaBox".to_string())));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(0))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(200))));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(200))));
        assert_eq!(pdf.next(), Some(Token::ArrayEnd));
        assert_eq!(pdf.next(), Some(Token::Name("Count".to_string())));
        assert_eq!(pdf.next(), Some(Token::Numeric(Number::Integer(1))));
        assert_eq!(pdf.next(), Some(Token::Name("Kids".to_string())));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(
            pdf.next(),
            Some(Token::IndirectRef((3, 0), &xref, &bytes.as_slice()))
        );
        assert_eq!(pdf.next(), Some(Token::ArrayEnd));
        assert_eq!(pdf.next(), Some(Token::DictEnd));
        assert_eq!(pdf.next(), Some(Token::ObjEnd));
    }
}
