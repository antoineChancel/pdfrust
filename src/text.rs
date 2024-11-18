use core::{iter::Iterator, panic};
use std::num::ParseIntError;

use crate::{
    body::FontMap,
    tokenizer::{Number, Token, Tokenizer},
};
struct Content<'a>(Tokenizer<'a>);

#[derive(Debug, PartialEq)]
enum ArrayVal {
    Text(String),
    Pos(Number),
}

type DashArray = Vec<Number>;
type DashPhase = Number;
type LineWidth = Number;
type LineStyle = Number;
type x = Number;
type y = Number;
type Gray = Number; // gray is a number between 0.0 (black) and 1.0 (white)

#[derive(Debug, PartialEq)]
enum GraphicsInstruction {
    // Graphic state operators (page 219)
    q,
    Q,
    cm(Number, Number, Number, Number, Number, Number), // Modify current transfo matrix
    w(LineWidth), // Set the line width in the graphics state
    J(LineStyle), // Set the line cap style in the graphics state
    d(DashArray, DashPhase), // Set the line dash pattern in the graphics state
    i(Number), // Set the flatness tolerance in the graphics state
    // Path construction operators (page 226)
    m(x, y), // Begin a new subpath by moving the current point to coordinates (x, y)
    l(x, y), // Append a straight line segment from the current point to the point (x, y). The new current point is (x, y).
    re(Number, Number, Number, Number), // Append a rectangle to the current path as a complete subpath, with lower-left corner (x, y) and dimensions width and height in user space.
    // Clipping paths operators (page 235)
    W,
    // Path painting operators (page 230)
    S,
    f,
    f_star, // Fill the path, using the even-odd rule to determine the region to fill
    n,
    // Color operators (page 287)
    cs(String),
    sc(Number),
    G(Gray), // // Set the stroking color space to DeviceGray
    g(Gray), // Same as G but used for nonstroking operations.
    // Text positionning operators (page 406)
    Td(Number, Number), // move to the start of next line
    TD(Number, Number), // move to the start of next line
    Tm(Number, Number, Number, Number, Number, Number), // set text matrix Tm and text line matrix Tlm
    // Text state operators (page 398)
    Tf(String, Number), // text font
    // Text-showing operators (page 407)
    Tj(String),        // show text string
    TJ(Vec<ArrayVal>), // show text array
    // Text object operator (page 405)
    BeginText,
    EndText,
    // XObject operator (page 332)
    Do(String),
}

impl<'a> From<Tokenizer<'a>> for Content<'a> {
    fn from(s: Tokenizer<'a>) -> Self {
        Content(s)
    }
}

impl<'a> From<&'a [u8]> for Content<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Content(Tokenizer::new(bytes, 0))
    }
}

impl Iterator for Content<'_> {
    type Item = GraphicsInstruction;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf: Vec<Token> = vec![];
        while let Some(t) = self.0.next() {
            match t {
                Token::LitteralString(_) => buf.push(t),
                Token::Name(_) => buf.push(t),
                Token::ArrayBegin => buf.push(t),
                Token::ArrayEnd => buf.push(t),
                Token::HexString(_) => buf.push(t),
                Token::Numeric(_) => buf.push(t),
                Token::String(l) => match l.as_slice() {
                    b"q" => return Some(GraphicsInstruction::q),
                    b"Q" => return Some(GraphicsInstruction::Q),
                    b"cm" => {
                        return Some(GraphicsInstruction::cm(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[1] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[2] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[3] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[4] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[5] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                        ))
                    }
                    b"w" => {
                        return Some(GraphicsInstruction::w(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator J"),
                            }))
                    }
                    b"J" => {
                        return Some(GraphicsInstruction::J(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator J"),
                            }))
                    }
                    b"d" => {
                        let mut e = buf.iter();
                        match e.next() {
                            Some(Token::ArrayBegin) => (),
                            Some(t) => panic!("First operand {t:?} is not allowed for operator d"),
                            None => panic!("End of stream too early"),
                        };
                        let mut dash_array = DashArray::new();
                        while let Some(t) = e.next() {
                            match t {
                                Token::Numeric(n) => dash_array.push(n.clone()),
                                Token::ArrayEnd => break,
                                t => panic!("Unexpected token {t:?} in dash array"),
                            }
                        }
                        let dash_phase = match e.next() {
                            Some(Token::Numeric(n)) => n.clone(),
                            Some(t) => panic!("First operand {t:?} is not allowed for operator d"),
                            None => panic!("End of stream too early"),
                        };
                        return Some(GraphicsInstruction::d(dash_array, dash_phase));
                    }
                    b"i" => {
                        return Some(GraphicsInstruction::i(match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        }))
                    }
                    b"m" => {
                        return Some(GraphicsInstruction::m(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[1] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                        ))
                    }
                    b"l" => {
                        return Some(GraphicsInstruction::l(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[1] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                        ))
                    }
                    b"re" => {
                        return Some(GraphicsInstruction::re(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[1] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[2] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                            match &buf[3] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator re"),
                            },
                        ))
                    }
                    b"W" => return Some(GraphicsInstruction::W),
                    b"n" => return Some(GraphicsInstruction::n),
                    b"S" => return Some(GraphicsInstruction::S),
                    b"f" => return Some(GraphicsInstruction::f),
                    b"f*" => return Some(GraphicsInstruction::f_star),
                    b"cs" => {
                        return Some(GraphicsInstruction::cs(match &buf[0] {
                            Token::Name(s) => s.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator cs"),
                        }))
                    }
                    b"sc" => {
                        return Some(GraphicsInstruction::sc(match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator cs"),
                        }))
                    }
                    b"G" => {
                        return Some(GraphicsInstruction::G(match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator G"),
                        }))
                    }
                    b"g" => {
                        return Some(GraphicsInstruction::g(match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator G"),
                        }))
                    }
                    b"BT" => return Some(GraphicsInstruction::BeginText),
                    b"ET" => return Some(GraphicsInstruction::EndText),
                    b"Tj" => {
                        return Some(GraphicsInstruction::Tj(match &buf[0] {
                            Token::LitteralString(l) => String::from_utf8(l.to_vec()).unwrap(),
                            t => panic!("Operand {t:?} is not allowed with operator Tj"),
                        }))
                    }
                    b"TD" => {
                        return Some(GraphicsInstruction::TD(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator TD"),
                            },
                            match &buf[1] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator TD"),
                            },
                        ))
                    }
                    b"Td" => {
                        if buf.len() == 2 {
                            return Some(GraphicsInstruction::Td(
                                match &buf[0] {
                                    Token::Numeric(n) => n.clone(),
                                    t => panic!("Operand {t:?} is not allowed with operator TD"),
                                },
                                match &buf[1] {
                                    Token::Numeric(n) => n.clone(),
                                    t => panic!("Operand {t:?} is not allowed with operator TD"),
                                },
                            ))
                        } else { // skip
                            return self.next()
                        }
                    }
                    b"Tf" => {
                        return Some(GraphicsInstruction::Tf(
                            match &buf[0] {
                                Token::Name(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator TD"),
                            },
                            match &buf[1] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator TD"),
                            },
                        ))
                    }
                    b"Tm" => {
                        return Some(GraphicsInstruction::Tm(
                            match &buf[0] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator Tm"),
                            },
                            match &buf[1] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator Tm"),
                            },
                            match &buf[2] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator Tm"),
                            },
                            match &buf[3] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator Tm"),
                            },
                            match &buf[4] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator Tm"),
                            },
                            match &buf[5] {
                                Token::Numeric(n) => n.clone(),
                                t => panic!("Operand {t:?} is not allowed with operator Tm"),
                            },
                        ))
                    }
                    b"TJ" => {
                        return Some(GraphicsInstruction::TJ(
                            buf.iter()
                                .filter(|t| {
                                    matches!(
                                        t,
                                        Token::LitteralString(_)
                                            | Token::String(_)
                                            | Token::Numeric(_)
                                    )
                                })
                                .map(|t| match t {
                                    Token::LitteralString(s) => {
                                        ArrayVal::Text(String::from_utf8(s.to_vec()).unwrap())
                                    }
                                    Token::String(s) => {
                                        ArrayVal::Text(String::from_utf8(s.to_vec()).unwrap())
                                    }
                                    Token::Numeric(n) => ArrayVal::Pos(n.clone()),
                                    t => panic!("Impossible {t:?}"),
                                })
                                .collect(),
                        ))
                    }
                    b"Do" => {
                        return Some(GraphicsInstruction::Do(match &buf[0] {
                            Token::Name(s) => s.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Do"),
                        }))
                    }
                    s => println!(
                        "Content token operator {:?} is not known, operands {:?}",
                        String::from_utf8(s.to_vec()),
                        buf
                    ),
                },
                t => panic!("Pdf token {t:?} has no mapping implemented to ContentStream"),
            }
        }
        None
    }
}

pub struct TextContent {
    text: String,
}

impl From<&[u8]> for TextContent {
    fn from(value: &[u8]) -> Self {
        let instructions = Content::from(Tokenizer::new(value, 0));
        TextContent {
            text: instructions
                .filter(|e| matches!(e, GraphicsInstruction::Tj(_) | GraphicsInstruction::TJ(_)))
                .map(|i| match i {
                    GraphicsInstruction::Tj(s) => s,
                    GraphicsInstruction::TJ(v) => v
                        .iter()
                        .filter(|e| matches!(e, ArrayVal::Text(_)))
                        .map(|s| match s {
                            ArrayVal::Text(s) => s.clone(),
                            _ => String::new(),
                        })
                        .collect::<Vec<String>>()
                        .join(""),
                    _ => String::new(),
                })
                .collect::<Vec<String>>()
                .join("\n"),
        }
    }
}

impl TextContent {
    pub fn decode_hex(hexstring: &str) -> Result<Vec<u8>, ParseIntError> {
        (0..hexstring.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hexstring[i..i + 2], 16))
            .collect()
    }

    pub fn get_text(&self, fontmap: FontMap) -> String {
        self.text.clone()
        // self.text
        //     .iter()
        //     .map(|t| {
        //         // collect font informations for the current text object content
        //         let font = match &t.t_f {
        //             Some(text_font) => fontmap.0.get(&text_font.0),
        //             None => None,
        //         };
        //         if let Some(ref v) = t.t_upper_j {
        //             return v
        //                 .iter()
        //                 .map(|elem| match elem {
        //                     PdfString::Litteral(s) => s.clone(),
        //                     PdfString::HexString(s) => {
        //                         let hex_bytes = Content::decode_hex(s).expect("Unable to decode hexstring to bytes");
        //                         let mut s= String::new();
        //                         match font {
        //                             Some(f) => {
        //                                 // if to unicode mapping exists, hex characters are mapped
        //                                 if let Some(to_unicode) = &f.to_unicode {
        //                                     for char_key in hex_bytes {
        //                                         let char_key = char_key as usize;
        //                                         match to_unicode.0.get(&char_key) {
        //                                             Some(char_val) => s.push(*char_val),
        //                                             None => {
        //                                                 panic!("Char with hex code {char_key} was not found")
        //                                             }
        //                                         }
        //                                     }
        //                                 } else {
        //                                     for c in hex_bytes {
        //                                         s.push(char::from(c))
        //                                     }
        //                                 }
        //                             }
        //                             None => for c in hex_bytes {
        //                                 s.push(char::from(c))
        //                             },
        //                         };
        //                         s
        //                     }
        //                 })
        //                 .collect::<Vec<String>>()
        //                 .join("");
        //         };
        //         match &t.t_j {
        //             Some(s) => s.clone() + "\n",
        //             // Text does not contains TJ or Tj operator
        //             None => "".to_string(),
        //         }
        //     })
        //     .collect()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_tokens() {
        let raw = b"BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET".as_slice();
        let mut stream = Content::from(raw);
        assert_eq!(stream.next(), Some(GraphicsInstruction::BeginText));
        assert_eq!(
            stream.next(),
            Some(GraphicsInstruction::TD(
                Number::Integer(70),
                Number::Integer(50)
            ))
        );
        assert_eq!(
            stream.next(),
            Some(GraphicsInstruction::Tf(
                "F1".to_string(),
                Number::Integer(12)
            ))
        );
        assert_eq!(
            stream.next(),
            Some(GraphicsInstruction::Tj("Hello, world!".to_string()))
        );
        assert_eq!(stream.next(), Some(GraphicsInstruction::EndText));
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_stream_hexstrings() {
        let raw = b"[<18>14<0D>2<06>7<14>1<04>-4<03>21<02>1<06>-2<04>-4<02>1<0906>]TJ".as_slice();
        let mut stream = Content::from(raw);
        // assert_eq!(stream.next(), Some(GraphicsInstruction::TJ(Vec)));
    }

    #[test]
    fn test_text_single() {
        let raw = b"BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET".as_slice();
        let text = TextContent::from(raw);
        assert_eq!(text.text, "Hello, world!".to_string());
    }

    // #[test]
    // fn test_text_hexstrings() {
    //     let raw =  b"BT\n56.8 706.189 Td /F1 10 Tf[<18>14<0D>2<06>7<14>1<04>-4<03>21<02>1<06>-2<04>-4<02>1<0906>]TJ\nET".as_slice();
    //     let text = Text::from(raw);
    //     assert_eq!(text.t_d, Some((Number::Real(56.8), Number::Real(706.189))));
    //     assert_eq!(text.t_f, Some(("F1".to_string(), Number::Integer(10))));
    //     assert_eq!(
    //         text.t_upper_j,
    //         Some(vec![
    //             PdfString::HexString("18".to_string()),
    //             PdfString::HexString("0D".to_string()),
    //             PdfString::HexString("06".to_string()),
    //             PdfString::HexString("14".to_string()),
    //             PdfString::HexString("04".to_string()),
    //             PdfString::HexString("03".to_string()),
    //             PdfString::HexString("02".to_string()),
    //             PdfString::HexString("06".to_string()),
    //             PdfString::HexString("04".to_string()),
    //             PdfString::HexString("02".to_string()),
    //             PdfString::HexString("0906".to_string())
    //         ])
    //     );
    // }

    //     #[test]
    //     fn test_text_multiple() {
    //         let raw = b"BT 12 0 0 -12 72 688 Tm /F3.0 1 Tf [ (eget)
    // -27 ( ) -30 (dui.) 47 ( ) -104 (Phasellus) -43 ( ) -13 (congue.) 42 ( ) -99
    // (Aenean) 54 ( ) -111 (est) -65 ( ) 8 (erat,) 29 ( ) -86 (tincidunt) -54 ( )
    // -3 (eget,) 31 ( ) -88 (venenatis) 5 ( ) -62 (quis,) 61 ( ) -118 (commodo)
    // -11 ( ) -46 (at, ) ] TJ ET"
    //             .as_slice();
    //         let text = Content::from(raw);
    //         assert_eq!(
    //             text.t_m,
    //             Some((
    //                 Number::Integer(12),
    //                 Number::Integer(0),
    //                 Number::Integer(0),
    //                 Number::Integer(-12),
    //                 Number::Integer(72),
    //                 Number::Integer(688)
    //             ))
    //         );
    //         assert_eq!(text.t_f, Some(("F3.0".to_string(), Number::Integer(1))));
    //         assert_eq!(
    //             text.t_upper_j,
    //             Some(
    //                 vec![
    //                     "eget",
    //                     " ",
    //                     "dui.",
    //                     " ",
    //                     "Phasellus",
    //                     " ",
    //                     "congue.",
    //                     " ",
    //                     "Aenean",
    //                     " ",
    //                     "est",
    //                     " ",
    //                     "erat,",
    //                     " ",
    //                     "tincidunt",
    //                     " ",
    //                     "eget,",
    //                     " ",
    //                     "venenatis",
    //                     " ",
    //                     "quis,",
    //                     " ",
    //                     "commodo",
    //                     " ",
    //                     "at, "
    //                 ]
    //                 .iter()
    //                 .map(|s| PdfString::Litteral(s.to_string()))
    //                 .collect()
    //             )
    //         );
    //     }

    //     #[test]
    //     fn test_content_stream() {
    //         let raw = b"q Q q 0 0 612 792 re W n /Cs1 cs 1 sc 0 0 612 792 re f 0.6000000 i 0 0 612 792
    // re f 0.3019608 sc 0 i q 1 0 0 -1 0 792 cm BT 36 0 0 -36 72 106 Tm /F1.0 1
    // Tf (Sample PDF) Tj ET Q 0 sc q 1 0 0 -1 0 792 cm BT 18 0 0 -18 72 132 Tm /F2.0
    // 1 Tf (This is a simple PDF file. Fun fun fun.) Tj ET Q q 1 0 0 -1 0 792 cm
    // BT 12 0 0 -12 72 163 Tm /F3.0 1 Tf [ (Lor) 17 (em) -91 ( ) -35 (ipsum) -77
    // ( ) -49 (dolor) 12 ( ) -139 (sit) -38 ( ) -89 (amet,) 61 ( ) -188 (consectetuer)
    // -5 ( ) -122 (adipiscing) -35 ( ) -91 (elit.) -1 ( ) -125 (Phasellus) -23 ( )
    // -103 (facilisis) -37 ( ) -89 (odio) -12 ( ) -114 (sed) -34 ( ) -93 (mi. )
    // ] TJ ET Q q 1 0 0 -1 0 792 cm BT 12 0 0 -12 72 178 Tm /F3.0 1 Tf [ (Curabitur)
    // -18 ( ) -41 (suscipit.) 21 ( ) -82 (Nullam) -94 ( ) 34 (vel) -6 ( ) -53 (nisi.)
    // -3 ( ) -57 (Etiam) -73 ( ) 12 (semper) 5 ( ) -65 (ipsum) -47 ( ) -13 (ut)
    // -43 ( ) -16 (lectus.) 25 ( ) -86 (Pr) 17 (oin) 68 ( ) -128 (aliquam,) 35 ( )
    // -96 (erat) -61 ( eget ) ] TJ ET Q q 1 0 0 -1"
    //             .as_slice();
    //         let text = Content::from(raw);
    //         assert_eq!(text.text.len(), 4);
    //         assert_eq!(text.get_text(FontMap::default()), "Sample PDF\nThis is a simple PDF file. Fun fun fun.\nLorem ipsum dolor sit amet, consectetuer adipiscing elit. Phasellus facilisis odio sed mi. Curabitur suscipit. Nullam vel nisi. Etiam semper ipsum ut lectus. Proin aliquam, erat eget ");
    //     }

    //     #[test]
    //     fn test_tokenizer_complex() {
    //         let raw = b"BT\n/F33 8.9664 Tf 54 713.7733 Td[(v0)-525(:=)-525(ld)-525(state[748])-2625(//)-525(load)-525(primes)-525(from)-525(the)-525(trace)-525(activation)-525(record)]TJ".as_slice();
    //         let mut text_stream = ContentStream::from(raw);
    //         assert_eq!(text_stream.next(), Some(ContentToken::BeginText));
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Name("F33".to_string()))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Real(8.9664)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Operator(Operator::Tf))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(54)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Real(713.7733)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Operator(Operator::Td))
    //         );
    //         assert_eq!(text_stream.next(), Some(ContentToken::BeginArray));
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from("v0")))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from(":=")))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from("ld")))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from(
    //                 "state[748]".to_string()
    //             )))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-2625)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from("//".to_string())))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from("load".to_string())))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from(
    //                 "primes".to_string()
    //             )))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from("from".to_string())))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from("the".to_string())))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from("trace".to_string())))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from(
    //                 "activation".to_string()
    //             )))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Numeric(Number::Integer(-525)))
    //         );
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::LitteralString(Vec::from(
    //                 "record".to_string()
    //             )))
    //         );
    //         assert_eq!(text_stream.next(), Some(ContentToken::EndArray));
    //         assert_eq!(
    //             text_stream.next(),
    //             Some(ContentToken::Operator(Operator::TJ))
    //         );
    //     }
}
