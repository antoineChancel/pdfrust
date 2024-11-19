use core::panic;
use std::{iter::Peekable, ops::Neg, slice::Iter};

use crate::xref::XrefTable;

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

#[derive(Debug, PartialEq, Clone)]
pub enum Number {
    Integer(i32),
    Real(f32),
}

impl From<Number> for f32 {
    fn from(value: Number) -> Self {
        match value {
            Number::Integer(i) => i as f32,
            Number::Real(f) => f,
        }
    }
}

impl std::ops::Neg for Number {
    type Output = Number;
    fn neg(self) -> Self::Output {
        match self {
            Number::Integer(i) => Number::Integer(-i),
            Number::Real(f) => Number::Real(-f),
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
    IndirectRef((i32, i32), &'a XrefTable, &'a [u8]),
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
                                // litteral
                                Some(_) => {
                                    let mut buf: Vec<u8> = vec![];
                                    loop {
                                        match self.byte.next() {
                                            Some(b'>') => break,
                                            Some(a) => buf.push(*a),
                                            None => panic!(
                                                "Reached end of stream before enf of litteral"
                                            ),
                                        }
                                    }
                                    return Some(Token::HexString(buf));
                                }
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
                    Delimiter::String => {
                        let mut buf: Vec<u8> = vec![];
                        // nested parentesis counters
                        let mut opened_parathesis: u8 = 1;
                        let mut closed_parathesis: u8 = 0;
                        for cursor in self.byte.by_ref() {
                            if let CharacterSet::Delimiter(Delimiter::String) =
                                CharacterSet::from(cursor)
                            {
                                if *cursor == b'(' {
                                    opened_parathesis += 1;
                                } else if *cursor == b')' {
                                    closed_parathesis += 1;
                                }
                                if opened_parathesis == closed_parathesis {
                                    break;
                                }
                            }
                            buf.push(*cursor);
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
                        match numeric.parse::<i32>() {
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
                b"4E6F762073686D6F7A206B6120706F702E".to_vec()
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
