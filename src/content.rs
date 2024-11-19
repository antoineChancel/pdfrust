use core::iter::Iterator;
use std::num::ParseIntError;

use crate::{
    algebra::Matrix,
    body::FontMap,
    tokenizer::{Number, Token, Tokenizer},
};

#[derive(Default)]
struct TextObject {
    tm: Matrix,  // text matrix
    tlm: Matrix, // text line matrix
}

struct Content<'a> {
    graphic_state: GraphicsState,
    graphic_state_stack: Vec<GraphicsState>,
    text_object: TextObject,
    tokenizer: Tokenizer<'a>,
}

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
type r = Number;
type g = Number;
type b = Number;

#[derive(Debug, PartialEq)]
enum GraphicsInstruction {
    // Graphic state operators (page 219)
    q,
    Q,
    cm(Number, Number, Number, Number, Number, Number), // Modify current transfo matrix
    w(LineWidth),                                       // Set the line width in the graphics state
    J(LineStyle),            // Set the line cap style in the graphics state
    d(DashArray, DashPhase), // Set the line dash pattern in the graphics state
    i(Number),               // Set the flatness tolerance in the graphics state
    // Path construction operators (page 226)
    m(x, y), // Begin a new subpath by moving the current point to coordinates (x, y)
    l(x, y), // Append a straight line segment from the current point to the point (x, y). The new current point is (x, y).
    re(Number, Number, Number, Number), // Append a rectangle to the current path as a complete subpath, with lower-left corner (x, y) and dimensions width and height in user space.
    // Clipping paths operators (page 235)
    W,
    W_star,
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
    rg(r, g, b),
    // Text positionning operators (page 406)
    Td(Number, Number), // move to the start of next line
    TD(Number, Number), // move to the start of next line
    Tm(Number, Number, Number, Number, Number, Number), // set text matrix Tm and text line matrix Tlm
    T_star,
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
    fn from(tokenizer: Tokenizer<'a>) -> Self {
        Content {
            graphic_state: GraphicsState::default(),
            graphic_state_stack: vec![],
            text_object: TextObject::default(),
            tokenizer,
        }
    }
}

impl<'a> From<&'a [u8]> for Content<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Content {
            graphic_state: GraphicsState::default(),
            graphic_state_stack: vec![],
            text_object: TextObject::default(),
            tokenizer: Tokenizer::new(bytes, 0),
        }
    }
}

impl Content<'_> {
    fn process_q(&mut self) {
        self.graphic_state_stack.push(self.graphic_state.clone())
    }

    fn process_Q(&mut self) {
        self.graphic_state = self
            .graphic_state_stack
            .pop()
            .expect("Unable to restore graphic state from empty stack");
    }

    fn process_cm(&mut self, cm: [Number; 6]) {
        self.graphic_state.ctm = cm;
    }

    fn process_w(&mut self, line_width: Number) {
        self.graphic_state.line_width = line_width;
    }

    fn process_J(&mut self, line_cap: Number) {
        self.graphic_state.line_cap = line_cap;
    }

    fn process_d(&mut self, dash_array: DashArray) {}

    fn process_i(&mut self, flatness: Number) {
        self.graphic_state.flatness = flatness;
    }

    fn process_m(&mut self, x: Number, y: Number) {}

    fn process_l(&mut self, x: Number, y: Number) {}

    fn process_re(&mut self, x: Number, y: Number, width: Number, height: Number) {}

    fn process_BT(&mut self) {
        self.graphic_state.text_state = TextState::default();
    }

    fn process_Td(&mut self, tx: Number, ty: Number) {
        self.text_object.tlm =
            Matrix::new(1.0, 0.0, 0.0, 1.0, f32::from(tx), f32::from(ty)) * self.text_object.tlm;
        self.text_object.tm = self.text_object.tlm;
    }

    fn process_TD(&mut self, tx: Number, ty: Number) {
        self.graphic_state.text_state.Tl = -ty.clone();
        self.process_Td(tx, ty);
    }

    fn process_Tf(&mut self, font: String, size: Number) {
        self.graphic_state.text_state.Tf = Some(font);
        self.graphic_state.text_state.Tfs = Some(size);
    }

    fn process_Tm(&mut self, a: Number, b: Number, c: Number, d: Number, e: Number, f: Number) {
        self.text_object.tm = Matrix::new(
            f32::from(a.clone()),
            f32::from(b.clone()),
            f32::from(c.clone()),
            f32::from(d.clone()),
            f32::from(e.clone()),
            f32::from(f.clone()),
        );
        self.text_object.tlm = Matrix::new(
            f32::from(a),
            f32::from(b),
            f32::from(c),
            f32::from(d),
            f32::from(e),
            f32::from(f),
        );
    }

    fn process_T_star(&mut self) {
        self.process_Td(Number::Integer(0), self.graphic_state.text_state.Tl.clone());
    }
}

impl Iterator for Content<'_> {
    type Item = GraphicsInstruction;

    fn next(&mut self) -> Option<Self::Item> {
        let mut buf: Vec<Token> = vec![];
        while let Some(t) = self.tokenizer.next() {
            match t {
                Token::LitteralString(_) => buf.push(t),
                Token::Name(_) => buf.push(t),
                Token::ArrayBegin => buf.push(t),
                Token::ArrayEnd => buf.push(t),
                Token::HexString(_) => buf.push(t),
                Token::Numeric(_) => buf.push(t),
                Token::String(l) => match l.as_slice() {
                    b"q" => {
                        self.process_q();
                        return Some(GraphicsInstruction::q);
                    }
                    b"Q" => {
                        self.process_Q();
                        return Some(GraphicsInstruction::Q);
                    }
                    b"cm" => {
                        let a = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let b = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let c = match &buf[2] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let d = match &buf[3] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let e = match &buf[4] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let f = match &buf[5] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        self.process_cm([
                            a.clone(),
                            b.clone(),
                            c.clone(),
                            d.clone(),
                            e.clone(),
                            f.clone(),
                        ]);
                        return Some(GraphicsInstruction::cm(a, b, c, d, e, f));
                    }
                    b"w" => {
                        let line_width = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator J"),
                        };
                        self.process_w(line_width.clone());
                        return Some(GraphicsInstruction::w(line_width));
                    }
                    b"J" => {
                        let line_cap = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator J"),
                        };
                        self.process_J(line_cap.clone());
                        return Some(GraphicsInstruction::J(line_cap));
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
                        self.process_d(dash_array.clone());
                        return Some(GraphicsInstruction::d(dash_array, dash_phase));
                    }
                    b"i" => {
                        let flatness = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        self.process_i(flatness.clone());
                        return Some(GraphicsInstruction::i(flatness));
                    }
                    b"m" => {
                        let x = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let y = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        self.process_m(x.clone(), y.clone());
                        return Some(GraphicsInstruction::m(x, y));
                    }
                    b"l" => {
                        let x = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let y = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        self.process_l(x.clone(), y.clone());
                        return Some(GraphicsInstruction::l(x, y));
                    }
                    b"re" => {
                        let x = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let y = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let width = match &buf[2] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        let height = match &buf[3] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        self.process_re(x.clone(), y.clone(), width.clone(), height.clone());
                        return Some(GraphicsInstruction::re(x, y, width, height));
                    }
                    b"W" => return Some(GraphicsInstruction::W),
                    b"W*" => return Some(GraphicsInstruction::W_star),
                    b"S" => return Some(GraphicsInstruction::S),
                    b"f" => return Some(GraphicsInstruction::f),
                    b"f*" => return Some(GraphicsInstruction::f_star),
                    b"n" => return Some(GraphicsInstruction::n),
                    b"cs" => {
                        let color_space = match &buf[0] {
                            Token::Name(s) => s.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator cs"),
                        };
                        return Some(GraphicsInstruction::cs(color_space));
                    }
                    b"sc" => {
                        let colors = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator cs"),
                        };
                        return Some(GraphicsInstruction::sc(colors));
                    }
                    b"G" => {
                        let gray = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator G"),
                        };
                        return Some(GraphicsInstruction::G(gray));
                    }
                    b"g" => {
                        let gray = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator G"),
                        };
                        return Some(GraphicsInstruction::g(gray));
                    }
                    b"rg" => {
                        let r = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator rg"),
                        };
                        let g = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator rg"),
                        };
                        let b = match &buf[2] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator rg"),
                        };
                        return Some(GraphicsInstruction::rg(r, g, b));
                    }
                    b"BT" => {
                        self.process_BT();
                        return Some(GraphicsInstruction::BeginText);
                    }
                    b"ET" => return Some(GraphicsInstruction::EndText),
                    b"Tj" => {
                        let text = match &buf[0] {
                            Token::LitteralString(l) => String::from_utf8(l.to_vec()).unwrap(),
                            t => panic!("Operand {t:?} is not allowed with operator Tj"),
                        };
                        return Some(GraphicsInstruction::Tj(text));
                    }
                    b"TD" => {
                        let tx = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        let ty = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        self.process_TD(tx.clone(), ty.clone());
                        return Some(GraphicsInstruction::TD(tx, ty));
                    }
                    b"Td" => {
                        if buf.len() != 2 {
                            return self.next();
                        }
                        let tx = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        let ty = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        self.process_Td(tx.clone(), ty.clone());
                        return Some(GraphicsInstruction::Td(tx, ty));
                    }
                    b"Tf" => {
                        let font = match &buf[0] {
                            Token::Name(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        let size = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        self.process_Tf(font.clone(), size.clone());
                        return Some(GraphicsInstruction::Tf(font, size));
                    }
                    b"Tm" => {
                        let a = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let b = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let c = match &buf[2] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let d = match &buf[3] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let e = match &buf[4] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let f = match &buf[5] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        self.process_Tm(
                            a.clone(),
                            b.clone(),
                            c.clone(),
                            d.clone(),
                            e.clone(),
                            f.clone(),
                        );
                        return Some(GraphicsInstruction::Tm(a, b, c, d, e, f));
                    }
                    b"T*" => {
                        self.process_T_star();
                        return Some(GraphicsInstruction::T_star);
                    }
                    b"TJ" => {
                        return Some(GraphicsInstruction::TJ(
                            buf.iter()
                                .filter(|t| {
                                    matches!(
                                        t,
                                        Token::LitteralString(_)
                                            | Token::HexString(_)
                                            | Token::Numeric(_)
                                    )
                                })
                                .map(|t| match t {
                                    Token::LitteralString(s) => {
                                        ArrayVal::Text(String::from_utf8(s.to_vec()).unwrap())
                                    }
                                    Token::HexString(s) => {
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

// Text state operators (page 397)
#[derive(Clone)]
struct TextState {
    Tc: Number,          // char spacing
    Tw: Number,          // word spacing
    Th: Number,          // horizontal scaling
    Tl: Number,          // leading
    Tf: Option<String>,  // text font
    Tfs: Option<Number>, // text font size
    Tmode: Number,       // text rendering mode
    Trise: Number,       // text rise
    Tk: Option<Number>,  // text knockout
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            Tc: Number::Integer(0),
            Tw: Number::Integer(0),
            Th: Number::Real(1.0),
            Tl: Number::Integer(0),
            Tf: None,
            Tfs: None,
            Tmode: Number::Integer(0),
            Trise: Number::Integer(0),
            Tk: None,
        }
    }
}

#[derive(Clone)]
struct GraphicsState {
    // device-independant state
    ctm: [Number; 6], // current transformation matrix
    // TODO: clipping_path,
    color_space: String, // current color space
    // TODO: color,
    text_state: TextState,
    line_width: Number,
    line_cap: Number,
    line_join: Number,
    miter_limit: Number,
    // TODO: dash_pattern,
    rendering_intent: String,
    stroke_adjustment: bool,
    blend_mode: String,
    // TODO: softmask,
    alpha_constant: Number,
    alpha_source: bool,
    // device dependant state
    overprint: bool,
    overprint_mode: Number,
    // TODO: black_generation,
    // TODO: undercolor_removal
    // TODO: transfer
    // TODO: halftone
    flatness: Number,
    // TODO: smoothness: Number
}

impl Default for GraphicsState {
    fn default() -> Self {
        Self {
            ctm: [
                Number::Integer(1),
                Number::Integer(0),
                Number::Integer(0),
                Number::Integer(1),
                Number::Integer(0),
                Number::Integer(0),
            ],
            color_space: String::from("DeviceGray"),
            text_state: TextState::default(),
            line_width: Number::Real(1.0),
            line_cap: Number::Integer(0), // square butt caps
            line_join: Number::Integer(0),
            miter_limit: Number::Real(10.0),
            rendering_intent: String::from("RelativeColorimetric"),
            stroke_adjustment: false,
            blend_mode: String::from("Normal"),
            alpha_constant: Number::Real(1.0),
            alpha_source: false,
            overprint: false,
            overprint_mode: Number::Integer(0),
            flatness: Number::Real(1.0),
        }
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

    use crate::object::Array;

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

    #[test]
    fn test_tokenizer_complex() {
        let raw = b"BT\n/F33 8.9664 Tf 54 713.7733 Td[(v0)-525(:=)-525(ld)-525(state[748])-2625(//)-525(load)-525(primes)-525(from)-525(the)-525(trace)-525(activation)-525(record)]TJ".as_slice();
        let mut text_stream = Content::from(raw);
        assert_eq!(text_stream.next(), Some(GraphicsInstruction::BeginText));
        assert_eq!(
            text_stream.next(),
            Some(GraphicsInstruction::Tf(
                "F33".to_string(),
                Number::Real(8.9664)
            ))
        );
        assert_eq!(
            text_stream.next(),
            Some(GraphicsInstruction::Td(
                Number::Integer(54),
                Number::Real(713.7733)
            ))
        );
        assert_eq!(
            text_stream.next(),
            Some(GraphicsInstruction::TJ(vec![
                ArrayVal::Text("v0".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(":=".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("ld".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("state[748]".to_string()),
                ArrayVal::Pos(Number::Integer(-2625)),
                ArrayVal::Text("//".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("load".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("primes".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("from".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("the".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("trace".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("activation".to_string()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text("record".to_string()),
                ]))
        );
        assert_eq!(text_stream.next(), None);
    }
}
