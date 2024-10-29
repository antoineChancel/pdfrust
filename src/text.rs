use core::iter::Iterator;
use std::{iter::Peekable, slice::Iter};

use crate::tokenizer::{CharacterSet, Delimiter};
struct Stream<'a>(Peekable<Iter<'a, u8>>);

#[derive(Debug, PartialEq)]
enum Operator {
    // text positionning
    Td,
    TD,
    Tm,
    // text font
    Tf,
    // text showing
    Tj, // single
    TJ, // multiple
}

#[derive(Debug, PartialEq)]
enum StreamToken {
    BeginText,
    EndText,
    BeginArray,
    EndArray,
    Operator(Operator),
    Name(String),
    Numeric(f32),
    Text(String),
    Other(String),
}

impl<'a> From<&'a [u8]> for Stream<'a> {
    fn from(s: &'a [u8]) -> Self {
        Stream(s.iter().peekable())
    }
}

impl<'a> Iterator for Stream<'a> {
    type Item = StreamToken;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf = String::new();
        while let Some(c) = self.0.next() {
            match CharacterSet::from(c) {
                CharacterSet::WhiteSpace(_) => continue,
                CharacterSet::Delimiter(d) => match d {
                    Delimiter::String => {
                        while let Some(c) = self.0.next() {
                            match CharacterSet::from(c) {
                                CharacterSet::Delimiter(Delimiter::String) => break,
                                _ => buf.push(*c as char),
                            }
                        }
                        return Some(StreamToken::Text(buf));
                    }
                    Delimiter::Name => {
                        while let Some(c) = self.0.next() {
                            match CharacterSet::from(c) {
                                CharacterSet::WhiteSpace(_) => break,
                                CharacterSet::Delimiter(_) => buf.push(*c as char),
                                CharacterSet::Regular(c) => buf.push(c as char),
                            }
                        }
                        return Some(StreamToken::Name(buf));
                    }
                    Delimiter::Array => match c {
                        b'[' => return Some(StreamToken::BeginArray),
                        b']' => return Some(StreamToken::EndArray),
                        b'<' => return Some(StreamToken::Other("<".to_string())),
                        b'{' => return Some(StreamToken::Other("{".to_string())),
                        b'}' => return Some(StreamToken::Other("}".to_string())),
                        b'>' => return Some(StreamToken::Other(">".to_string())),
                        c => panic!("Invalid character {}", *c as char),
                    },
                    _ => panic!("Invalid character"),
                },
                CharacterSet::Regular(c) => {
                    buf.push(c as char);
                    while let Some(c) = self.0.peek() {
                        match CharacterSet::from(*c) {
                            CharacterSet::WhiteSpace(_) => break,
                            CharacterSet::Regular(c) => {
                                self.0.next();
                                buf.push(c as char);
                            }
                            CharacterSet::Delimiter(_) => break,
                        }
                    }
                    return match buf.as_str() {
                        "BT" => Some(StreamToken::BeginText),
                        "ET" => Some(StreamToken::EndText),
                        "Tj" => Some(StreamToken::Operator(Operator::Tj)),
                        "TD" => Some(StreamToken::Operator(Operator::TD)),
                        "Td" => Some(StreamToken::Operator(Operator::Td)),
                        "Tf" => Some(StreamToken::Operator(Operator::Tf)),
                        "Tm" => Some(StreamToken::Operator(Operator::Tm)),
                        "TJ" => Some(StreamToken::Operator(Operator::TJ)),
                        _ => match buf.parse::<f32>() {
                            Ok(n) => Some(StreamToken::Numeric(n)),
                            Err(_) => Some(StreamToken::Other(buf)),
                        },
                    };
                }
            }
        }
        None
    }
}

#[derive(Debug, PartialEq)]
struct Text {
    t_upper_d: Option<(f32, f32)>, // Move text position and set leading
    t_d: Option<(f32, f32)>,       // Move text position
    t_m: Option<(f32, f32, f32, f32, f32, f32)>, // Set text matrix and text line matrix
    t_f: Option<(String, f32)>,    // Set text font and size
    t_j: Option<String>,           // Show text
    t_upper_j: Option<Vec<String>>, // Show text, allowing individual glyph positioning
}

impl<'a> From<&mut Stream<'a>> for Text {
    fn from(value: &mut Stream<'a>) -> Self {
        let mut text = Text {
            t_upper_d: None,
            t_d: None,
            t_m: None,
            t_f: None,
            t_j: None,
            t_upper_j: None,
        };
        let mut buf: Vec<StreamToken> = vec![];
        while let Some(token) = value.next() {
            match token {
                StreamToken::BeginText => continue,
                StreamToken::EndText => break,
                StreamToken::Operator(op) => match op {
                    Operator::Td => {
                        text.t_d = Some((
                            match buf[0] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token, buf {buf:?}"),
                            },
                            match buf[1] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                        ));
                        buf.clear();
                    }
                    Operator::TD => {
                        text.t_upper_d = Some((
                            match buf[0] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[1] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                        ));
                        buf.clear();
                    }
                    Operator::Tm => {
                        text.t_m = Some((
                            match buf[0] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[1] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[2] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[3] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[4] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[5] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                        ));
                        buf.clear();
                    }
                    Operator::Tf => {
                        text.t_f = Some((
                            match &buf[0] {
                                StreamToken::Name(n) => n.clone(),
                                StreamToken::Other(n) => n.clone(), // may happen
                                _ => panic!("Invalid token, buffer {buf:?}"),
                            },
                            match buf[1] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token {:?}", buf),
                            },
                        ));
                        buf.clear();
                    }
                    Operator::Tj => {
                        text.t_j = Some(match &buf[0] {
                            StreamToken::Text(n) => n.clone(),
                            _ => panic!("Invalid token"),
                        });
                        buf.clear();
                    }
                    Operator::TJ => {
                        text.t_upper_j = Some(
                            buf.iter()
                                .filter(|t| match t {
                                    StreamToken::Text(_) => true,
                                    _ => false,
                                })
                                .map(|f| match f {
                                    StreamToken::Text(t) => t.clone(),
                                    _ => panic!("Invalid token"),
                                })
                                .collect(),
                        );
                        buf.clear();
                    }
                },
                t => buf.push(t),
            }
        }
        text
    }
}

impl From<&[u8]> for Text {
    fn from(value: &[u8]) -> Self {
        Self::from(&mut Stream::from(value))
    }
}

pub struct StreamContent {
    text: Vec<Text>,
}

impl From<&[u8]> for StreamContent {
    fn from(value: &[u8]) -> Self {
        let mut stream_iter = Stream::from(value);
        let mut text = vec![];
        while let Some(token) = stream_iter.next() {
            match token {
                StreamToken::BeginText => {
                    text.push(Text::from(&mut stream_iter));
                }
                _ => continue,
            }
        }
        StreamContent { text }
    }
}

impl StreamContent {
    pub fn get_text(&self) -> String {
        self.text
            .iter()
            .map(|t| {
                match t.t_upper_j {
                    Some(ref v) => return v.join("") + "\n",
                    None => (),
                };
                match t.t_j {
                    Some(ref s) => return s.clone() + "\n",
                    // Text does not contains TJ or Tj operator
                    None => return "".to_string(),
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
        let mut stream_iter = Stream::from(raw);
        assert_eq!(stream_iter.next(), Some(StreamToken::BeginText));
        assert_eq!(stream_iter.next(), Some(StreamToken::Numeric(70.0)));
        assert_eq!(stream_iter.next(), Some(StreamToken::Numeric(50.0)));
        assert_eq!(
            stream_iter.next(),
            Some(StreamToken::Operator(Operator::TD))
        );
        assert_eq!(
            stream_iter.next(),
            Some(StreamToken::Name("F1".to_string()))
        );
        assert_eq!(stream_iter.next(), Some(StreamToken::Numeric(12.0)));
        assert_eq!(
            stream_iter.next(),
            Some(StreamToken::Operator(Operator::Tf))
        );
        assert_eq!(
            stream_iter.next(),
            Some(StreamToken::Text("Hello, world!".to_string()))
        );
        assert_eq!(
            stream_iter.next(),
            Some(StreamToken::Operator(Operator::Tj))
        );
        assert_eq!(stream_iter.next(), Some(StreamToken::EndText));
        assert_eq!(stream_iter.next(), None);
    }

    #[test]
    fn test_text_single() {
        let raw = b"BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET".as_slice();
        let text = Text::from(raw);
        assert_eq!(text.t_upper_d, Some((70.0, 50.0)));
        assert_eq!(text.t_f, Some(("F1".to_string(), 12.0)));
        assert_eq!(text.t_j, Some("Hello, world!".to_string()));
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
        assert_eq!(text.t_m, Some((12.0, 0.0, 0.0, -12.0, 72.0, 688.0)));
        assert_eq!(text.t_f, Some(("F3.0".to_string(), 1.0)));
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
                .map(|s| s.to_string())
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
        let text = StreamContent::from(raw);
        assert_eq!(text.text.len(), 4);
        assert_eq!(text.get_text(), "Sample PDF\nThis is a simple PDF file. Fun fun fun.\nLorem ipsum dolor sit amet, consectetuer adipiscing elit. Phasellus facilisis odio sed mi. \nCurabitur suscipit. Nullam vel nisi. Etiam semper ipsum ut lectus. Proin aliquam, erat eget \n");
    }

    #[test]
    fn test_tokenizer_complex() {
        let raw = b"BT\n/F33 8.9664 Tf 54 713.7733 Td[(v0)-525(:=)-525(ld)-525(state[748])-2625(//)-525(load)-525(primes)-525(from)-525(the)-525(trace)-525(activation)-525(record)]TJ".as_slice();
        let mut text_stream = Stream::from(raw);
        assert_eq!(text_stream.next(), Some(StreamToken::BeginText));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Name("F33".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(8.9664)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Operator(Operator::Tf))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(54.0)));
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(713.7733)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Operator(Operator::Td))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::BeginArray));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("v0".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text(":=".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("ld".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("state[748]".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-2625.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("//".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("load".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("primes".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("from".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("the".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("trace".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("activation".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::Numeric(-525.0)));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Text("record".to_string()))
        );
        assert_eq!(text_stream.next(), Some(StreamToken::EndArray));
        assert_eq!(
            text_stream.next(),
            Some(StreamToken::Operator(Operator::TJ))
        );
    }
}
