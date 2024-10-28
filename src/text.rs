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
    Operator(Operator),
    Name(String),
    Numeric(u32),
    Text(String),
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
                CharacterSet::Delimiter(Delimiter::String) => {
                    while let Some(c) = self.0.next() {
                        match CharacterSet::from(c) {
                            CharacterSet::Delimiter(Delimiter::String) => break,
                            _ => buf.push(*c as char),
                        }
                    }
                    return Some(StreamToken::Text(buf));
                }
                CharacterSet::Delimiter(Delimiter::Name) => {
                    while let Some(c) = self.0.next() {
                        match CharacterSet::from(c) {
                            CharacterSet::WhiteSpace(_) => break,
                            CharacterSet::Delimiter(_) => panic!("Invalid character"),
                            CharacterSet::Regular(c) => buf.push(c as char),
                        }
                    }
                    return Some(StreamToken::Name(buf));
                }
                CharacterSet::Delimiter(c) => panic!("Character {c:?} is not covered"),
                CharacterSet::Regular(c) => {
                    buf.push(c as char);
                    while let Some(c) = self.0.next() {
                        match CharacterSet::from(c) {
                            CharacterSet::WhiteSpace(_) => break,
                            CharacterSet::Regular(c) => buf.push(c as char),
                            CharacterSet::Delimiter(_) => panic!("Invalid character"),
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
                        _ => match buf.parse::<u32>() {
                            Ok(n) => Some(StreamToken::Numeric(n)),
                            Err(_) => None,
                        },
                    };
                }
            }
        }
        None
    }
}

struct Text {
    TD: Option<(u32, u32)>,
    Td: Option<(u32, u32)>,
    Tm: Option<(u32, u32, u32, u32, u32, u32)>,
    Tf: Option<(String, u32)>,
    Tj: Option<String>,
    TJ: Option<Vec<String>>,
}

impl From<&[u8]> for Text {
    fn from(value: &[u8]) -> Self {
        let mut stream_iter = Stream::from(value);
        let mut text = Text {
            TD: None,
            Td: None,
            Tm: None,
            Tf: None,
            Tj: None,
            TJ: None,
        };
        let mut buf: Vec<StreamToken> = vec![];
        while let Some(token) = stream_iter.next() {
            match token {
                StreamToken::BeginText => continue,
                StreamToken::EndText => break,
                StreamToken::Operator(op) => match op {
                    Operator::Td => {
                        text.Td = Some((
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
                    Operator::TD => {
                        text.TD = Some((
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
                        text.Tm = Some((
                            match buf[0] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[1] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[0] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                            match buf[1] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
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
                    Operator::Tf => {
                        text.Tf = Some((
                            match &buf[0] {
                                StreamToken::Name(n) => n.clone(),
                                _ => panic!("Invalid token"),
                            },
                            match buf[1] {
                                StreamToken::Numeric(n) => n,
                                _ => panic!("Invalid token"),
                            },
                        ));
                        buf.clear();
                    }
                    Operator::Tj => {
                        text.Tj = Some(match &buf[0] {
                            StreamToken::Text(n) => n.clone(),
                            _ => panic!("Invalid token"),
                        });
                        buf.clear();
                    }
                    Operator::TJ => {
                        text.TJ = Some(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text() {
        let raw = b"BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET".as_slice();
        let text = Text::from(raw);
        assert_eq!(text.TD, Some((70, 50)));
        assert_eq!(text.Tf, Some(("F1".to_string(), 12)));
        assert_eq!(text.Tj, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_tokens() {
        let raw = b"BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET".as_slice();
        let mut stream_iter = Stream::from(raw);
        assert_eq!(stream_iter.next(), Some(StreamToken::BeginText));
        assert_eq!(stream_iter.next(), Some(StreamToken::Numeric(70)));
        assert_eq!(stream_iter.next(), Some(StreamToken::Numeric(50)));
        assert_eq!(
            stream_iter.next(),
            Some(StreamToken::Operator(Operator::TD))
        );
        assert_eq!(
            stream_iter.next(),
            Some(StreamToken::Name("F1".to_string()))
        );
        assert_eq!(stream_iter.next(), Some(StreamToken::Numeric(12)));
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
}
