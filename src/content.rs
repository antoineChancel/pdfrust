use core::iter::Iterator;

use crate::{
    algebra::{Matrix, Number},
    body::Resources,
    object::Name,
    tokenizer::{Token, Tokenizer},
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
    Text(Vec<u8>),
    Pos(Number),
}

type DashArray = Vec<Number>;
type DashPhase = Number;
type LineWidth = Number;
type LineStyle = Number;
type X = Number;
type Y = Number;
type X1 = Number;
type Y1 = Number;
type X2 = Number;
type Y2 = Number;
type X3 = Number;
type Y3 = Number;
type Gray = Number; // gray is a number between 0.0 (black) and 1.0 (white)
type R = Number;
type G = Number;
type B = Number;

#[derive(Debug, PartialEq)]
enum GraphicsInstruction {
    // Graphic state operators (page 219)
    LowerQ,
    UpperQ,
    BDC, // Structure content operator (page 850) -> ignored at the moment
    BMC,
    EMC,
    Cm(Number, Number, Number, Number, Number, Number), // Modify current transfo matrix
    LowerW(LineWidth),                                  // Set the line width in the graphics state
    UpperJ(LineStyle),            // Set the line cap style in the graphics state
    LowerD(DashArray, DashPhase), // Set the line dash pattern in the graphics state
    LowerI(Number),               // Set the flatness tolerance in the graphics state
    Gs,                           // Set the specified parameters in the graphics state
    // Path construction operators (page 226)
    LowerM(X, Y), // Begin a new subpath by moving the current point to coordinates (x, y)
    LowerL(X, Y), // Append a straight line segment from the current point to the point (x, y). The new current point is (x, y)
    LowerC(X1, Y1, X2, Y2, X3, Y3), // Append a cubic Bézier curve to the current path
    LowerH, // Close the current subpath by appending a straight line segment from the current point to the starting point of the subpath
    Re(Number, Number, Number, Number), // Append a rectangle to the current path as a complete subpath, with lower-left corner (x, y) and dimensions width and height in user space.
    // Clipping paths operators (page 235)
    W,
    WStar,
    // Path painting operators (page 230)
    S,
    LowerF,
    LowerFStar, // Fill the path, using the even-odd rule to determine the region to fill
    N,
    // Color operators (page 287)
    Cs(String),
    Sc(Number),
    UpperG(Gray),               // Set the stroking color space to DeviceGray
    LowerG(Gray),               // Same as G but used for nonstroking operations
    RG(Number, Number, Number), // Set the stroking color space to DeviceRGB and set the color intensities
    Rg(R, G, B),
    // Text positionning operators (page 406)
    Td(Number, Number), // move to the start of next line
    TD(Number, Number), // move to the start of next line
    Tm(Number, Number, Number, Number, Number, Number), // set text matrix Tm and text line matrix Tlm
    TStar,
    // Text state operators (page 398)
    Tc(Number),         // set char space
    Tf(String, Number), // set text font
    Tr(Number),         // set text mode
    // Text-showing operators (page 407)
    Tj(Vec<u8>),       // show text string
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

    fn process_upper_q(&mut self) {
        self.graphic_state = self
            .graphic_state_stack
            .pop()
            .expect("Unable to restore graphic state from empty stack");
    }

    fn process_cm(&mut self, cm: [Number; 6]) {
        self.graphic_state.ctm = Matrix::from(cm);
    }

    fn process_w(&mut self, line_width: Number) {
        self.graphic_state.line_width = line_width;
    }

    fn process_upper_j(&mut self, line_cap: Number) {
        self.graphic_state.line_cap = line_cap;
    }

    fn process_d(&mut self, _dash_array: DashArray) {}

    fn process_i(&mut self, flatness: Number) {
        self.graphic_state.flatness = flatness;
    }

    fn process_gs(&mut self, _dict_name: Name) {}

    fn process_m(&mut self, _x: Number, _y: Number) {}

    fn process_l(&mut self, _x: Number, _y: Number) {}

    fn process_c(&mut self, _x1: Number, _y1: Number, _x2: Number, _y2: Number, _x3: Number, _y3: Number) {}

    fn process_re(&mut self, _x: Number, _y: Number, _width: Number, _height: Number) {}

    fn process_bt(&mut self) {
        self.text_object = TextObject::default();
    }

    fn process_tc(&mut self, tc: Number) {
        self.graphic_state.text_state.tc = tc;
    }

    fn process_td(&mut self, tx: Number, ty: Number) {
        self.text_object.tlm =
            Matrix::new(1.0, 0.0, 0.0, 1.0, f32::from(tx), f32::from(ty)) * self.text_object.tlm;
        self.text_object.tm = self.text_object.tlm;
    }

    fn process_t_upper_d(&mut self, tx: Number, ty: Number) {
        self.graphic_state.text_state.tl = -ty.clone();
        self.process_td(tx, ty);
    }

    fn process_tr(&mut self, render: Number) {
        self.graphic_state.text_state.tmode = render;
    }

    fn process_tf(&mut self, font: String, size: Number) {
        self.graphic_state.text_state.tf = Some(font);
        self.graphic_state.text_state.tfs = Some(size);
    }

    fn process_tm(&mut self, a: Number, b: Number, c: Number, d: Number, e: Number, f: Number) {
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

    fn process_t_star(&mut self) {
        self.process_td(Number::Integer(0), self.graphic_state.text_state.tl.clone());
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
                Token::DictBegin => {
                    // ignored for now
                    for t in self.tokenizer.by_ref() {
                        if t == Token::DictEnd {
                            break;
                        }
                    }
                }
                Token::HexString(_) => buf.push(t),
                Token::Numeric(_) => buf.push(t),
                Token::String(l) => match l.as_slice() {
                    b"q" => {
                        self.process_q();
                        return Some(GraphicsInstruction::LowerQ);
                    }
                    b"Q" => {
                        self.process_upper_q();
                        return Some(GraphicsInstruction::UpperQ);
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
                        return Some(GraphicsInstruction::Cm(a, b, c, d, e, f));
                    }
                    b"w" => {
                        let line_width = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator J"),
                        };
                        self.process_w(line_width.clone());
                        return Some(GraphicsInstruction::LowerW(line_width));
                    }
                    b"J" => {
                        let line_cap = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator J"),
                        };
                        self.process_upper_j(line_cap.clone());
                        return Some(GraphicsInstruction::UpperJ(line_cap));
                    }
                    b"d" => {
                        let mut e = buf.iter();
                        match e.next() {
                            Some(Token::ArrayBegin) => (),
                            Some(t) => panic!("First operand {t:?} is not allowed for operator d"),
                            None => panic!("End of stream too early"),
                        };
                        let mut dash_array = DashArray::new();
                        for t in e.by_ref() {
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
                        return Some(GraphicsInstruction::LowerD(dash_array, dash_phase));
                    }
                    b"i" => {
                        let flatness = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator re"),
                        };
                        self.process_i(flatness.clone());
                        return Some(GraphicsInstruction::LowerI(flatness));
                    }
                    b"gs" => {
                        let dict_name = match &buf[0] {
                            Token::Name(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator gs"),
                        };
                        self.process_gs(dict_name);
                        return Some(GraphicsInstruction::Gs);
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
                        return Some(GraphicsInstruction::LowerM(x, y));
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
                        return Some(GraphicsInstruction::LowerL(x, y));
                    }
                    b"c" => {
                        let x1 = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let y1 = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let x2 = match &buf[2] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let y2 = match &buf[3] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let x3 = match &buf[4] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        let y3 = match &buf[5] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tm"),
                        };
                        self.process_c(
                            x1.clone(),
                            y1.clone(),
                            x2.clone(),
                            y2.clone(),
                            x3.clone(),
                            y3.clone(),
                        );
                        return Some(GraphicsInstruction::LowerC(x1, y1, x2, y2, x3, y3));
                    }
                    b"h" => {
                        return Some(GraphicsInstruction::LowerH);
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
                        return Some(GraphicsInstruction::Re(x, y, width, height));
                    }
                    b"W" => return Some(GraphicsInstruction::W),
                    b"W*" => return Some(GraphicsInstruction::WStar),
                    b"S" => return Some(GraphicsInstruction::S),
                    b"f" => return Some(GraphicsInstruction::LowerF),
                    b"f*" => return Some(GraphicsInstruction::LowerFStar),
                    b"n" => return Some(GraphicsInstruction::N),
                    b"cs" => {
                        let color_space = match &buf[0] {
                            Token::Name(s) => s.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator cs"),
                        };
                        return Some(GraphicsInstruction::Cs(color_space));
                    }
                    b"sc" => {
                        let colors = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator cs"),
                        };
                        return Some(GraphicsInstruction::Sc(colors));
                    }
                    b"G" => {
                        let gray = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator G"),
                        };
                        return Some(GraphicsInstruction::UpperG(gray));
                    }
                    b"g" => {
                        let gray = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator G"),
                        };
                        return Some(GraphicsInstruction::LowerG(gray));
                    }
                    b"RG" => {
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
                        return Some(GraphicsInstruction::RG(r, g, b));
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
                        return Some(GraphicsInstruction::Rg(r, g, b));
                    }
                    b"BT" => {
                        self.process_bt();
                        return Some(GraphicsInstruction::BeginText);
                    }
                    b"ET" => return Some(GraphicsInstruction::EndText),
                    b"TD" => {
                        let tx = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        let ty = match &buf[1] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        self.process_t_upper_d(tx.clone(), ty.clone());
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
                        self.process_td(tx.clone(), ty.clone());
                        return Some(GraphicsInstruction::Td(tx, ty));
                    }
                    b"Tc" => {
                        let char_space = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator TD"),
                        };
                        self.process_tc(char_space.clone());
                        return Some(GraphicsInstruction::Tc(char_space));
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
                        self.process_tf(font.clone(), size.clone());
                        return Some(GraphicsInstruction::Tf(font, size));
                    }
                    b"Tr" => {
                        let render = match &buf[0] {
                            Token::Numeric(n) => n.clone(),
                            t => panic!("Operand {t:?} is not allowed with operator Tr"),
                        };
                        self.process_tr(render.clone());
                        return Some(GraphicsInstruction::Tr(render))
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
                        self.process_tm(
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
                        self.process_t_star();
                        return Some(GraphicsInstruction::TStar);
                    }
                    b"Tj" => {
                        let text = match &buf[0] {
                            Token::LitteralString(l) => l,
                            t => panic!("Operand {t:?} is not allowed with operator Tj"),
                        };
                        return Some(GraphicsInstruction::Tj(text.to_vec()));
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
                                    Token::LitteralString(s) => ArrayVal::Text(s.to_vec()),
                                    Token::HexString(s) => ArrayVal::Text(s.to_vec()),
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
                    b"BDC" => return Some(GraphicsInstruction::BDC),
                    b"BMC" => return Some(GraphicsInstruction::BMC),
                    b"EMC" => return Some(GraphicsInstruction::EMC),
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
    tc: Number,         // char spacing
    tw: Number,         // word spacing
    th: Number,         // horizontal scaling
    tl: Number,         // leading
    tf: Option<String>, // text font
    tfs: Option<Number>, // text font size
    tmode: Number,       // text rendering mode
                        // trise: Number,       // text rise
                        // tk: bool,            // text knockout
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            tc: Number::Integer(0),
            tw: Number::Integer(0),
            th: Number::Real(1.0),
            tl: Number::Integer(0),
            tf: None,
            tfs: None,
            tmode: Number::Integer(0),
            // trise: Number::Integer(0),
            // tk: true,
        }
    }
}

#[derive(Clone)]
struct GraphicsState {
    // device-independant state
    ctm: Matrix, // current transformation matrix
    // TODO: clipping_path,
    // color_space: String, // current color space
    // TODO: color,
    text_state: TextState,
    line_width: Number,
    line_cap: Number,
    // line_join: Number,
    // miter_limit: Number,
    // TODO: dash_pattern,
    // rendering_intent: String,
    // stroke_adjustment: bool,
    // blend_mode: String,
    // TODO: softmask,
    // alpha_constant: Number,
    // alpha_source: bool,
    // device dependant state
    // overprint: bool,
    // overprint_mode: Number,
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
            ctm: Matrix::default(), // identity matrix
            // color_space: String::from("DeviceGray"),
            text_state: TextState::default(),
            line_width: Number::Real(1.0),
            line_cap: Number::Integer(0), // square butt caps
            // line_join: Number::Integer(0),
            // miter_limit: Number::Real(10.0),
            // rendering_intent: String::from("RelativeColorimetric"),
            // stroke_adjustment: false,
            // blend_mode: String::from("Normal"),
            // alpha_constant: Number::Real(1.0),
            // alpha_source: false,
            // overprint: false,
            // overprint_mode: Number::Integer(0),
            flatness: Number::Real(1.0),
        }
    }
}

pub struct TextContent<'a> {
    resources: Box<Resources>,
    content: Content<'a>,
}

impl<'a> TextContent<'a> {
    pub fn new(content_bytes: &'a [u8], resources: Box<Resources>) -> Self {
        Self {
            resources,
            content: Content::from(Tokenizer::new(content_bytes, 0)),
        }
    }

    pub fn get_text(&mut self, display_char: bool) -> String {
        let mut output = String::new();
        while let Some(i) = self.content.next() {
            match i {
                GraphicsInstruction::Tj(text) => {
                    let font = match self.content.graphic_state.text_state.tf {
                        Some(ref s) => match &self.resources.font {
                            Some(fontmap) => fontmap.0.get(s).unwrap(),
                            None => panic!("Fontmap does not contains the font name {s:?}"),
                        },
                        None => panic!("Text state should have a font set"),
                    };
                    for c in text {
                        if display_char {
                            output += format!(
                                "{:?}, {:?}, {:?}, {:}\n",
                                c as char,
                                font.subtype,
                                font.base_font,
                                self.content.text_object.tm
                            )
                            .as_str();
                        } else {
                            output.push(c as char);
                        }
                    }
                    output.push('\n');
                }
                GraphicsInstruction::TJ(text) => {
                    // current font
                    let font = match self.content.graphic_state.text_state.tf {
                        Some(ref s) => match &self.resources.font {
                            Some(fontmap) => fontmap.0.get(s).unwrap(),
                            None => panic!("Fontmap does not contains the font name {s:?}"),
                        },
                        None => panic!("Text state should have a font set"),
                    };

                    // estimate current space width (before scaling)
                    // let w_space = font.estimate_space_width();
                    // println!("{text:?}");
                    // println!("Space width: {w_space:?}");
                    for c in text {
                        match c {
                            ArrayVal::Text(t) => {
                                // string characters in to unicode map
                                match &font.to_unicode {
                                    Some(to_unicode_cmap) => {
                                        let mut hex_iter = t.iter();
                                        while let Some(c) = hex_iter.next() {
                                            let char_idx = match to_unicode_cmap.is_two_bytes {
                                                true => {
                                                    *c as usize * 256
                                                        + *hex_iter.next().unwrap() as usize
                                                }
                                                false => usize::from(*c),
                                            };
                                            let char = match to_unicode_cmap.cmap.get(&char_idx) {
                                                Some(c) => c,
                                                None => panic!("CMap does not contain a char with idx {:?}, charmap {:?}", char_idx, to_unicode_cmap.cmap)
                                            };
                                            // paint glyph
                                            if display_char {
                                                output += format!(
                                                    "{:?}, {:?}, {:?}, {:}\n",
                                                    char,
                                                    font.subtype,
                                                    font.base_font,
                                                    self.content.text_object.tm
                                                )
                                                .as_str();
                                            } else {
                                                output.push(*char);
                                            }
                                            // println!("{:?}", *to_unicode_cmap.0.get(&usize::from(c)).unwrap());
                                            // displacement vector
                                            let w0: Number = match font.clone().get_width(*c) {
                                                Ok(n) => n,
                                                Err(_) => Number::Real(0.0), // assumption at the moment...
                                            };
                                            // let w1 = Number::Integer(0); // temporary, need to be updated with writing mode (horizontal writing only)
                                            let tfs = match &self.content.graphic_state.text_state.tfs {
                                                Some(n) => n,
                                                None => panic!("Font size should be set before painting a glyph")
                                            };
                                            let tc =
                                                self.content.graphic_state.text_state.tc.clone();
                                            let tw =
                                                self.content.graphic_state.text_state.tw.clone();
                                            let th =
                                                self.content.graphic_state.text_state.th.clone();
                                            // update text matrix (page 410)
                                            // translation vector coordinates
                                            // tj displacement factor is added according to the text writing mode (assumed 0 for now) -> page 408
                                            let mut tx = w0.clone() * tfs.clone() + tc.clone();
                                            // tw displacement for word space
                                            if *c == b' ' {
                                                tx = tx + tw.clone();
                                            }
                                            tx = tx * th;
                                            let ty = Number::Real(0.0);
                                            // let ty = (w1.clone()) // + -tj.clone() / Number::Real(1000.0)) -> to be added if a different writing mode is selected
                                            //     * tfs.clone()
                                            //     + tc.clone()
                                            //     + tw.clone();
                                            // println!("ctm: {:?}", self.content.graphic_state.ctm);
                                            // println!("tx:{tx:?}, ty:{ty:?}, w0: {w0:?}, w1:{w1:?}, tfs:{tfs:?}, tc:{tc:?}, tw:{tw:?}");
                                            self.content.text_object.tm =
                                                Matrix::new(
                                                    1.0,
                                                    0.0,
                                                    0.0,
                                                    1.0,
                                                    tx.into(),
                                                    ty.into(),
                                                ) * self.content.text_object.tm;
                                            // println!("tm {:?}", self.content.text_object.tm);
                                        }
                                    }
                                    // no unicode mapping -> read as char
                                    None => {
                                        for c in t {
                                            if display_char {
                                                output += format!(
                                                    "{:?}, {:?}, {:?}, {:}\n",
                                                    c as char,
                                                    font.subtype,
                                                    font.base_font,
                                                    self.content.text_object.tm
                                                )
                                                .as_str();
                                            } else {
                                                output.push(c as char);
                                            }
                                            // println!("{:?}", c as char);
                                            // displacement vector
                                            let w0: Number = match font.clone().get_width(c) {
                                                Ok(w) => w,
                                                Err(_) => Number::Real(0.0)
                                            };
                                            // let w1 = Number::Integer(0); // temporary, need to be updated with writing mode (horizontal writing only)
                                            let tfs = match &self.content.graphic_state.text_state.tfs {
                                                Some(n) => n,
                                                None => panic!("Font size should be set before painting a glyph")
                                            };
                                            let tc =
                                                self.content.graphic_state.text_state.tc.clone();
                                            let tw =
                                                self.content.graphic_state.text_state.tw.clone();
                                            let th =
                                                self.content.graphic_state.text_state.th.clone();
                                            // update text matrix (page 410)
                                            // translation vector coordinates is (tx, ty)
                                            let mut tx = w0.clone() * tfs.clone() + tc.clone();
                                            // tw displacement for word space
                                            if c == b' ' {
                                                tx = tx + tw.clone();
                                            }
                                            tx = tx * th;
                                            let ty = Number::Real(0.0);
                                            // let ty = (w1.clone()) // + -tj.clone() / Number::Real(1000.0))
                                            //     * tfs.clone()
                                            //     + tc.clone()
                                            //     + tw.clone();
                                            // println!("ctm: {:?}", self.content.graphic_state.ctm);
                                            // println!("tx:{tx:?}, ty:{ty:?}, w0: {w0:?}, w1:{w1:?}, tfs:{tfs:?}, tc:{tc:?}, tw:{tw:?}");
                                            self.content.text_object.tm =
                                                Matrix::new(
                                                    1.0,
                                                    0.0,
                                                    0.0,
                                                    1.0,
                                                    tx.into(),
                                                    ty.into(),
                                                ) * self.content.text_object.tm;
                                            // println!("tm {:?}", self.content.text_object.tm);
                                        }
                                    }
                                };
                            }
                            // translation according to text writing direction (assumed horizontal for now)
                            ArrayVal::Pos(tj) => {
                                let tfs = match &self.content.graphic_state.text_state.tfs {
                                    Some(n) => n,
                                    None => {
                                        panic!("Font size should be set before painting a glyph")
                                    }
                                };
                                let th = self.content.graphic_state.text_state.th.clone();
                                let tx = -tj / Number::Real(1000.0) * tfs.clone() * th.clone();
                                // apply
                                // println!("Space width threshold {:?}", w_space.clone() * tfs.clone() * th);
                                // println!("Translate {:?}", tx);
                                self.content.text_object.tm =
                                    Matrix::new(1.0, 0.0, 0.0, 1.0, tx.clone().into(), 0.0)
                                        * self.content.text_object.tm;
                                // whitespace detected
                                // to be improved...
                                // match tx {
                                //     Number::Integer(i) => {
                                //         if i > 0 && !output.ends_with(' ') {
                                //             output.push(' ');
                                //         }
                                //     }
                                //     Number::Real(f) => {
                                //         if f > 0.0 && !output.ends_with(' ') {
                                //             output.push(' ');
                                //         }
                                //     }
                                // }
                            }
                        }
                    }
                    output.push('\n');
                }
                _ => (),
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {

    use std::vec;

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
            Some(GraphicsInstruction::Tj(b"Hello, world!".to_vec()))
        );
        assert_eq!(stream.next(), Some(GraphicsInstruction::EndText));
        assert_eq!(stream.next(), None);
    }

    #[test]
    fn test_stream_hexstrings() {
        let raw = b"[<18>14<0D>2<06>7<14>1<04>-4<03>21<02>1<06>-2<04>-4<02>1<0906>]TJ".as_slice();
        let mut stream = Content::from(raw);
        assert_eq!(
            stream.next(),
            Some(GraphicsInstruction::TJ(vec![
                ArrayVal::Text(vec![24]),
                ArrayVal::Pos(Number::Integer(14)),
                ArrayVal::Text(vec![13]),
                ArrayVal::Pos(Number::Integer(2)),
                ArrayVal::Text(vec![6]),
                ArrayVal::Pos(Number::Integer(7)),
                ArrayVal::Text(vec![20]),
                ArrayVal::Pos(Number::Integer(1)),
                ArrayVal::Text(vec![4]),
                ArrayVal::Pos(Number::Integer(-4)),
                ArrayVal::Text(vec![3]),
                ArrayVal::Pos(Number::Integer(21)),
                ArrayVal::Text(vec![2]),
                ArrayVal::Pos(Number::Integer(1)),
                ArrayVal::Text(vec![6]),
                ArrayVal::Pos(Number::Integer(-2)),
                ArrayVal::Text(vec![4]),
                ArrayVal::Pos(Number::Integer(-4)),
                ArrayVal::Text(vec![2]),
                ArrayVal::Pos(Number::Integer(1)),
                ArrayVal::Text(vec![9, 6]),
            ]))
        );
    }

    #[test]
    fn test_tokenizer_dict() {
        let raw = b" /P <</MCID 0>> BDC q\n0.00000887 0 595.25 842 re".as_slice();
        let mut text_stream = Content::from(raw);
        assert_eq!(text_stream.next(), Some(GraphicsInstruction::BDC));
        assert_eq!(text_stream.next(), Some(GraphicsInstruction::LowerQ));
        assert_eq!(
            text_stream.next(),
            Some(GraphicsInstruction::Re(
                Number::Real(0.00000887),
                Number::Integer(0),
                Number::Real(595.25),
                Number::Integer(842)
            ))
        );
    }

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
                ArrayVal::Text(b"v0".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b":=".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"ld".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"state[748]".to_vec()),
                ArrayVal::Pos(Number::Integer(-2625)),
                ArrayVal::Text(b"//".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"load".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"primes".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"from".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"the".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"trace".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"activation".to_vec()),
                ArrayVal::Pos(Number::Integer(-525)),
                ArrayVal::Text(b"record".to_vec()),
            ]))
        );
        assert_eq!(text_stream.next(), None);
    }
}
