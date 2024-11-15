use core::iter::Iterator;
use std::num::ParseIntError;

use crate::{
    body::FontMap, tokenizer::{Number, Token, Tokenizer}
};
struct ContentStream<'a>(Tokenizer<'a>);

#[derive(Debug, PartialEq)]
enum Operator {
    Td, // move to the start of next line
    TD, // move to the start of next line
    Tm, // set text matrix Tm and text line matrix Tlm
    Tf, // text font
    Tj, // show text string
    TJ, // show text array
}

#[derive(Debug, PartialEq)]
enum ContentToken {
    BeginText,
    EndText,
    BeginArray,
    EndArray,
    Operator(Operator),
    Name(String),
    Numeric(Number),
    LitteralString(Vec<u8>),
    HexString(Vec<u8>),
    Other(String),
}

impl<'a> From<Tokenizer<'a>> for ContentStream<'a> {
    fn from(s: Tokenizer<'a>) -> Self {
        ContentStream(s)
    }
}

impl<'a> From<&'a [u8]> for ContentStream<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        ContentStream(Tokenizer::new(bytes, 0))
    }
}

impl Iterator for ContentStream<'_> {
    type Item = ContentToken;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            Some(Token::LitteralString(l)) => Some(ContentToken::LitteralString(l)),
            Some(Token::Name(l)) => Some(ContentToken::Name(l)),
            Some(Token::ArrayBegin) => Some(ContentToken::BeginArray),
            Some(Token::ArrayEnd) => Some(ContentToken::EndArray),
            Some(Token::HexString(l)) => Some(ContentToken::HexString(l)),
            Some(Token::String(l)) => match l.as_slice() {
                b"BT" => Some(ContentToken::BeginText),
                b"ET" => Some(ContentToken::EndText),
                b"Tj" => Some(ContentToken::Operator(Operator::Tj)),
                b"TD" => Some(ContentToken::Operator(Operator::TD)),
                b"Td" => Some(ContentToken::Operator(Operator::Td)),
                b"Tf" => Some(ContentToken::Operator(Operator::Tf)),
                b"Tm" => Some(ContentToken::Operator(Operator::Tm)),
                b"TJ" => Some(ContentToken::Operator(Operator::TJ)),
                s => Some(ContentToken::Other(String::from(std::str::from_utf8(s).unwrap()))),
            },
            Some(Token::Numeric(n)) => Some(ContentToken::Numeric(n)),
            Some(t) => panic!("Pdf token {t:?} has no mapping implemented to ContentStream"),
            None => None,
        }
    }
}

#[derive(Debug, PartialEq)]
enum PdfString {
    Litteral(String),
    HexString(String),
}

#[derive(Debug, PartialEq)]
struct Text {
    t_upper_d: Option<(Number, Number)>, // Move text position and set leading
    t_d: Option<(Number, Number)>,       // Move text position
    t_m: Option<(Number, Number, Number, Number, Number, Number)>, // Set text matrix and text line matrix
    t_f: Option<(String, Number)>, // Set text font and size
    t_j: Option<String>, // Show text
    t_upper_j: Option<Vec<PdfString>>, // Show text, allowing individual glyph positioning
}

impl<'a> From<&mut ContentStream<'a>> for Text {
    fn from(value: &mut ContentStream<'a>) -> Self {
        let mut text = Text {
            t_upper_d: None,
            t_d: None,
            t_m: None,
            t_f: None,
            t_j: None,
            t_upper_j: None,
        };
        let mut operands: Vec<ContentToken> = vec![];
        for token in value.by_ref() {
            match token {
                ContentToken::BeginText => continue,
                ContentToken::EndText => break,
                ContentToken::Operator(op) => match op {
                    Operator::Td => {
                        text.t_d = Some((
                            match &operands[0] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Td operator"),
                            },
                            match &operands[1] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Td operator"),
                            },
                        ));
                        operands.clear();
                    }
                    Operator::TD => {
                        text.t_upper_d = Some((
                            match &operands[0] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for TD operator"),
                            },
                            match &operands[1] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for TD operator"),
                            },
                        ));
                        operands.clear();
                    }
                    Operator::Tm => {
                        text.t_m = Some((
                            match &operands[0] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Tm operator"),
                            },
                            match &operands[1] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Tm operator"),
                            },
                            match &operands[2] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Tm operator"),
                            },
                            match &operands[3] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Tm operator"),
                            },
                            match &operands[4] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Tm operator"),
                            },
                            match &operands[5] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Tm operator"),
                            },
                        ));
                        operands.clear();
                    }
                    Operator::Tf => {
                        text.t_f = Some((
                            match &operands[0] {
                                ContentToken::Name(n) => n.clone(),
                                ContentToken::Other(n) => n.clone(), // may happen
                                _ => panic!("Invalid operands {operands:?} for Tf operator"),
                            },
                            match &operands[1] {
                                ContentToken::Numeric(n) => n.clone(),
                                _ => panic!("Invalid operands {operands:?} for Tf operator"),
                            },
                        ));
                        operands.clear();
                    }
                    Operator::Tj => {
                        text.t_j = Some(match &operands[0] {
                            ContentToken::LitteralString(n) => {
                                String::from(std::str::from_utf8(n).unwrap())
                            }
                            _ => panic!("Invalid operands {operands:?} for Tj operator"),
                        });
                        operands.clear();
                    }
                    Operator::TJ => {
                        text.t_upper_j = Some(
                            operands.iter()
                                .filter(|t| {
                                    matches!(
                                        t,
                                        ContentToken::LitteralString(_) | ContentToken::HexString(_)
                                    )
                                })
                                .map(|f| match f {
                                    ContentToken::LitteralString(v) => PdfString::Litteral(
                                        String::from(std::str::from_utf8(v).unwrap()),
                                    ),
                                    ContentToken::HexString(v) => PdfString::HexString(
                                        String::from(std::str::from_utf8(v).unwrap()),
                                    ),
                                    _ => panic!("Invalid operands {operands:?} for TJ operator"),
                                })
                                .collect(),
                        );
                        operands.clear();
                    }
                },
                t => operands.push(t),
            }
        }
        text
    }
}

impl From<&[u8]> for Text {
    fn from(value: &[u8]) -> Self {
        Self::from(&mut ContentStream::from(Tokenizer::new(value, 0)))
    }
}

pub struct Content {
    text: Vec<Text>,
}

impl From<&[u8]> for Content {
    fn from(value: &[u8]) -> Self {
        let mut stream_iter = ContentStream::from(Tokenizer::new(value, 0));
        let mut text = vec![];
        while let Some(token) = stream_iter.next() {
            match token {
                ContentToken::BeginText => {
                    text.push(Text::from(&mut stream_iter));
                }
                _ => continue,
            }
        }
        Content { text }
    }
}

impl Content {
    pub fn decode_hex(hexstring: &str) -> Result<Vec<u8>, ParseIntError> {
        (0..hexstring.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hexstring[i..i + 2], 16))
            .collect()
    }

    pub fn get_text(&self, fontmap: FontMap) -> String {
        self.text
            .iter()
            .map(|t| {
                // collect font informations for the current text object content
                let font = match &t.t_f {
                    Some(text_font) => fontmap.0.get(&text_font.0),
                    None => None,
                };
                if let Some(ref v) = t.t_upper_j {
                    return v
                        .iter()
                        .map(|elem| match elem {
                            PdfString::Litteral(s) => s.clone(),
                            PdfString::HexString(s) => {
                                let hex_bytes = Content::decode_hex(s).expect("Unable to decode hexstring to bytes");
                                let mut s= String::new();
                                match font {
                                    Some(f) => {
                                        // if to unicode mapping exists, hex characters are mapped
                                        if let Some(to_unicode) = &f.to_unicode {
                                            for char_key in hex_bytes {
                                                let char_key = char_key as usize;
                                                match to_unicode.0.get(&char_key) {
                                                    Some(char_val) => s.push(*char_val),
                                                    None => {
                                                        panic!("Char with hex code {char_key} was not found")
                                                    }
                                                }
                                            }
                                        } else {
                                            for c in hex_bytes {
                                                s.push(char::from(c))
                                            }
                                        }
                                    }
                                    None => for c in hex_bytes {
                                        s.push(char::from(c))
                                    },
                                };
                                s
                            }
                        })
                        .collect::<Vec<String>>()
                        .join("");
                };
                match &t.t_j {
                    Some(s) => s.clone() + "\n",
                    // Text does not contains TJ or Tj operator
                    None => "".to_string(),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_tokens() {
        let raw = b"BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET".as_slice();
        let mut stream_iter = ContentStream::from(raw);
        assert_eq!(stream_iter.next(), Some(ContentToken::BeginText));
        assert_eq!(stream_iter.next(), Some(ContentToken::Numeric(Number::Integer(70))));
        assert_eq!(stream_iter.next(), Some(ContentToken::Numeric(Number::Integer(50))));
        assert_eq!(
            stream_iter.next(),
            Some(ContentToken::Operator(Operator::TD))
        );
        assert_eq!(
            stream_iter.next(),
            Some(ContentToken::Name("F1".to_string()))
        );
        assert_eq!(stream_iter.next(), Some(ContentToken::Numeric(Number::Integer(12))));
        assert_eq!(
            stream_iter.next(),
            Some(ContentToken::Operator(Operator::Tf))
        );
        assert_eq!(
            stream_iter.next(),
            Some(ContentToken::LitteralString(Vec::from(b"Hello, world!")))
        );
        assert_eq!(
            stream_iter.next(),
            Some(ContentToken::Operator(Operator::Tj))
        );
        assert_eq!(stream_iter.next(), Some(ContentToken::EndText));
        assert_eq!(stream_iter.next(), None);
    }

    #[test]
    fn test_stream_hexstrings() {
        let raw = b"[<18>14<0D>2<06>7<14>1<04>-4<03>21<02>1<06>-2<04>-4<02>1<0906>]TJ".as_slice();
        let mut stream = ContentStream::from(raw);
        assert_eq!(stream.next(), Some(ContentToken::BeginArray));
        assert_eq!(
            stream.next(),
            Some(ContentToken::HexString(Vec::from(b"18")))
        );
        assert_eq!(stream.next(), Some(ContentToken::Numeric(Number::Integer(14))));
        assert_eq!(
            stream.next(),
            Some(ContentToken::HexString(Vec::from("0D")))
        );
        assert_eq!(stream.next(), Some(ContentToken::Numeric(Number::Integer(2))));
    }

    #[test]
    fn test_text_single() {
        let raw = b"BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET".as_slice();
        let text = Text::from(raw);
        assert_eq!(text.t_upper_d, Some((Number::Integer(70), Number::Integer(50))));
        assert_eq!(text.t_f, Some(("F1".to_string(), Number::Integer(12))));
        assert_eq!(text.t_j, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_text_hexstrings() {
        let raw =  b"BT\n56.8 706.189 Td /F1 10 Tf[<18>14<0D>2<06>7<14>1<04>-4<03>21<02>1<06>-2<04>-4<02>1<0906>]TJ\nET".as_slice();
        let text = Text::from(raw);
        assert_eq!(text.t_d, Some((Number::Real(56.8), Number::Real(706.189))));
        assert_eq!(text.t_f, Some(("F1".to_string(), Number::Integer(10))));
        assert_eq!(
            text.t_upper_j,
            Some(vec![
                PdfString::HexString("18".to_string()),
                PdfString::HexString("0D".to_string()),
                PdfString::HexString("06".to_string()),
                PdfString::HexString("14".to_string()),
                PdfString::HexString("04".to_string()),
                PdfString::HexString("03".to_string()),
                PdfString::HexString("02".to_string()),
                PdfString::HexString("06".to_string()),
                PdfString::HexString("04".to_string()),
                PdfString::HexString("02".to_string()),
                PdfString::HexString("0906".to_string())
            ])
        );
    }

    #[test]
    fn test_text_multiple() {
        let raw = b"BT 12 0 0 -12 72 688 Tm /F3.0 1 Tf [ (eget)
-27 ( ) -30 (dui.) 47 ( ) -104 (Phasellus) -43 ( ) -13 (congue.) 42 ( ) -99
(Aenean) 54 ( ) -111 (est) -65 ( ) 8 (erat,) 29 ( ) -86 (tincidunt) -54 ( )
-3 (eget,) 31 ( ) -88 (venenatis) 5 ( ) -62 (quis,) 61 ( ) -118 (commodo)
-11 ( ) -46 (at, ) ] TJ ET"
            .as_slice();
        let text = Text::from(raw);
        assert_eq!(text.t_m, Some((Number::Integer(12), Number::Integer(0), Number::Integer(0), Number::Integer(-12), Number::Integer(72), Number::Integer(688))));
        assert_eq!(text.t_f, Some(("F3.0".to_string(), Number::Integer(1))));
        assert_eq!(
            text.t_upper_j,
            Some(
                vec![
                    "eget",
                    " ",
                    "dui.",
                    " ",
                    "Phasellus",
                    " ",
                    "congue.",
                    " ",
                    "Aenean",
                    " ",
                    "est",
                    " ",
                    "erat,",
                    " ",
                    "tincidunt",
                    " ",
                    "eget,",
                    " ",
                    "venenatis",
                    " ",
                    "quis,",
                    " ",
                    "commodo",
                    " ",
                    "at, "
                ]
                .iter()
                .map(|s| PdfString::Litteral(s.to_string()))
                .collect()
            )
        );
    }

    #[test]
    fn test_content_stream() {
        let raw = b"q Q q 0 0 612 792 re W n /Cs1 cs 1 sc 0 0 612 792 re f 0.6000000 i 0 0 612 792
re f 0.3019608 sc 0 i q 1 0 0 -1 0 792 cm BT 36 0 0 -36 72 106 Tm /F1.0 1
Tf (Sample PDF) Tj ET Q 0 sc q 1 0 0 -1 0 792 cm BT 18 0 0 -18 72 132 Tm /F2.0
1 Tf (This is a simple PDF file. Fun fun fun.) Tj ET Q q 1 0 0 -1 0 792 cm
BT 12 0 0 -12 72 163 Tm /F3.0 1 Tf [ (Lor) 17 (em) -91 ( ) -35 (ipsum) -77
( ) -49 (dolor) 12 ( ) -139 (sit) -38 ( ) -89 (amet,) 61 ( ) -188 (consectetuer)
-5 ( ) -122 (adipiscing) -35 ( ) -91 (elit.) -1 ( ) -125 (Phasellus) -23 ( )
-103 (facilisis) -37 ( ) -89 (odio) -12 ( ) -114 (sed) -34 ( ) -93 (mi. )
] TJ ET Q q 1 0 0 -1 0 792 cm BT 12 0 0 -12 72 178 Tm /F3.0 1 Tf [ (Curabitur)
-18 ( ) -41 (suscipit.) 21 ( ) -82 (Nullam) -94 ( ) 34 (vel) -6 ( ) -53 (nisi.)
-3 ( ) -57 (Etiam) -73 ( ) 12 (semper) 5 ( ) -65 (ipsum) -47 ( ) -13 (ut)
-43 ( ) -16 (lectus.) 25 ( ) -86 (Pr) 17 (oin) 68 ( ) -128 (aliquam,) 35 ( )
-96 (erat) -61 ( eget ) ] TJ ET Q q 1 0 0 -1"
            .as_slice();
        let text = Content::from(raw);
        assert_eq!(text.text.len(), 4);
        assert_eq!(text.get_text(FontMap::default()), "Sample PDF\nThis is a simple PDF file. Fun fun fun.\nLorem ipsum dolor sit amet, consectetuer adipiscing elit. Phasellus facilisis odio sed mi. Curabitur suscipit. Nullam vel nisi. Etiam semper ipsum ut lectus. Proin aliquam, erat eget ");
    }

    #[test]
    fn test_tokenizer_complex() {
        let raw = b"BT\n/F33 8.9664 Tf 54 713.7733 Td[(v0)-525(:=)-525(ld)-525(state[748])-2625(//)-525(load)-525(primes)-525(from)-525(the)-525(trace)-525(activation)-525(record)]TJ".as_slice();
        let mut text_stream = ContentStream::from(raw);
        assert_eq!(text_stream.next(), Some(ContentToken::BeginText));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::Name("F33".to_string()))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Real(8.9664))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::Operator(Operator::Tf))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(54))));
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Real(713.7733))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::Operator(Operator::Td))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::BeginArray));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("v0")))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from(":=")))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("ld")))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("state[748]".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-2625))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("//".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("load".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("primes".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("from".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("the".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("trace".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("activation".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::Numeric(Number::Integer(-525))));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::LitteralString(Vec::from("record".to_string())))
        );
        assert_eq!(text_stream.next(), Some(ContentToken::EndArray));
        assert_eq!(
            text_stream.next(),
            Some(ContentToken::Operator(Operator::TJ))
        );
    }
}
