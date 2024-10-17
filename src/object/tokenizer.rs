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
    Numeric(u32),
    String(&'a [u8]),
    LitteralString(&'a [u8]),
    Name(&'a str),
    Comment(&'a [u8]),
    IndirectRef(u32, u32),
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

pub struct Tokenizer<'a> {
    bytes: &'a [u8],
    curr_idx: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(bytes: &'a [u8]) -> Tokenizer<'a> {
        Tokenizer { bytes, curr_idx: 0 }
    }

    pub fn peek(&mut self, lap: usize) -> Option<Token<'a>> {
        let curr_idx = self.curr_idx;
        let mut token = None;
        for _ in 0..lap {
            token = self.next();
        }
        self.curr_idx = curr_idx;
        token
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token = None;
        loop {
            if self.curr_idx >= self.bytes.len() {
                break;
            }
            match CharacterSet::from(&self.bytes[self.curr_idx]) {
                CharacterSet::Delimiter(v) => match v {
                    Delimiter::Comment => {
                        // read all characters until a line feed or cariage return is met
                        let begin = self.curr_idx + 1;
                        loop {
                            self.curr_idx += 1;
                            // end of stream
                            if self.curr_idx >= self.bytes.len() {
                                break;
                            }
                            match CharacterSet::from(&self.bytes[self.curr_idx]) {
                                CharacterSet::WhiteSpace(WhiteSpace::CarriageReturn) => break,
                                CharacterSet::WhiteSpace(WhiteSpace::LineFeed) => break,
                                _ => (),
                            }
                        }
                        token = Some(Token::Comment(&self.bytes[begin..self.curr_idx]));
                        break;
                    }
                    Delimiter::Array => {
                        if &self.bytes[self.curr_idx..self.curr_idx + 2] == b"<<".as_ref() {
                            self.curr_idx += 2;
                            token = Some(Token::DictBegin);
                            break;
                        } else if &self.bytes[self.curr_idx..self.curr_idx + 2] == b">>".as_ref() {
                            self.curr_idx += 2;
                            token = Some(Token::DictEnd);
                            break;
                        } else if self.bytes[self.curr_idx] == b'[' {
                            self.curr_idx += 1;
                            token = Some(Token::ArrayBegin);
                            break;
                        } else if self.bytes[self.curr_idx] == b']' {
                            self.curr_idx += 1;
                            token = Some(Token::ArrayEnd);
                            break;
                        }
                    }
                    Delimiter::Name => {
                        let begin = self.curr_idx + 1;
                        loop {
                            self.curr_idx += 1;
                            // end of stream
                            if self.curr_idx >= self.bytes.len() {
                                break;
                            }
                            match CharacterSet::from(&self.bytes[self.curr_idx]) {
                                CharacterSet::Regular(_) => (),
                                _ => break,
                            }
                        }
                        token = Some(Token::Name(
                            std::str::from_utf8(&self.bytes[begin..self.curr_idx]).unwrap(),
                        ));
                        break;
                    }
                    // TODO: to be treated
                    Delimiter::String => {
                        let begin = self.curr_idx + 1;
                        let mut opened_parathesis: u8 = 1;
                        let mut closed_parathesis: u8 = 0;
                        loop {
                            self.curr_idx += 1;
                            // end of stream
                            if self.curr_idx >= self.bytes.len() {
                                break;
                            }
                            match CharacterSet::from(&self.bytes[self.curr_idx]) {
                                CharacterSet::Delimiter(Delimiter::String) => {
                                    if self.bytes[self.curr_idx] == b'(' {
                                        opened_parathesis += 1;
                                    } else if self.bytes[self.curr_idx] == b')' {
                                        closed_parathesis += 1;
                                    }
                                    if opened_parathesis == closed_parathesis {
                                        break;
                                    }
                                }
                                _ => (),
                            }
                        }
                        token = Some(Token::LitteralString(&self.bytes[begin..self.curr_idx]));
                        self.curr_idx += 1; // skip closing parenthesis
                        break;
                    }
                },
                // read regular string
                CharacterSet::Regular(_) => {
                    let begin = self.curr_idx;
                    let mut is_numeric = true;
                    loop {
                        match CharacterSet::from(&self.bytes[self.curr_idx]) {
                            CharacterSet::Regular(
                                b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9',
                            ) => (),
                            CharacterSet::Regular(_) => is_numeric = false,
                            _ => break,
                        }
                        self.curr_idx += 1;
                    }
                    if is_numeric {
                        let mut numeric = 0;
                        for c in &self.bytes[begin..self.curr_idx] {
                            numeric = numeric * 10 + char::from(*c).to_digit(10).unwrap()
                        }
                        token = Some(Token::Numeric(numeric));

                        // indirect reference (peek next 2 tokens)
                        match self.peek(1) {
                            Some(Token::Numeric(gen)) => match self.peek(2) {
                                Some(Token::String(b"R")) => {
                                    let obj = numeric;
                                    let gen = match self.next().unwrap() {
                                        Token::Numeric(gen) => gen,
                                        _ => panic!("Unable to read generation number"),
                                    };
                                    self.next(); // consume 'R'
                                    token = Some(Token::IndirectRef(obj, gen));
                                }
                                Some(Token::String(b"obj")) => {
                                    self.next(); // consume 'gen'
                                    self.next(); // consume 'R'
                                    token = Some(Token::ObjBegin);
                                }
                                _ => break,
                            },
                            _ => break,
                        }
                    } else if &self.bytes[begin..self.curr_idx] == b"stream" {
                        token = Some(Token::StreamBegin);
                    } else if &self.bytes[begin..self.curr_idx] == b"endstream" {
                        token = Some(Token::StreamEnd);
                    } else if &self.bytes[begin..self.curr_idx] == b"endobj" {
                        token = Some(Token::ObjEnd);
                    } else {
                        token = Some(Token::String(&self.bytes[begin..self.curr_idx]));
                    }
                    break;
                }
                // absorb whitespaces before a new token is met
                CharacterSet::WhiteSpace(_) => self.curr_idx += 1,
            }
        }
        match token {
            Some(Token::Comment(_)) => self.next(), //skip somment if any
            Some(token) => Some(token),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_pdfbytes_iterator_skipped_comment() {
        let mut pdf = Tokenizer::new(b"%PDF-1.7\n\n1 0 obj  % entry point");
        // comments are skipped by iterator
        assert_eq!(pdf.next(), Some(Token::ObjBegin));
    }

    #[test]
    fn test_pdfbytes_iterator_litteral_string() {
        let mut pdf = Tokenizer::new(b"(Hello World)");
        assert_eq!(pdf.next(), Some(Token::LitteralString(b"Hello World")));
    }

    #[test]
    fn test_pdfbytes_iterator_litteral_string_with_embedded_parenthesis() {
        let mut pdf = Tokenizer::new(b"((Hello) (World))");
        assert_eq!(pdf.next(), Some(Token::LitteralString(b"(Hello) (World)")));
    }

    #[test]
    fn test_pdfbytes_iterator_full() {
        let mut pdf = Tokenizer::new(b"2 0 obj\n<<\n  /Type /Pages\n  /MediaBox [ 0 0 200 200 ]\n  /Count 1\n  /Kids [ 3 0 R ]\n>>\nendobj\n");
        assert_eq!(pdf.next(), Some(Token::ObjBegin));
        assert_eq!(pdf.next(), Some(Token::DictBegin));
        assert_eq!(pdf.next(), Some(Token::Name("Type")));
        assert_eq!(pdf.next(), Some(Token::Name("Pages")));
        assert_eq!(pdf.next(), Some(Token::Name("MediaBox")));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::Numeric(0)));
        assert_eq!(pdf.next(), Some(Token::Numeric(0)));
        assert_eq!(pdf.next(), Some(Token::Numeric(200)));
        assert_eq!(pdf.next(), Some(Token::Numeric(200)));
        assert_eq!(pdf.next(), Some(Token::ArrayEnd));
        assert_eq!(pdf.next(), Some(Token::Name("Count")));
        assert_eq!(pdf.next(), Some(Token::Numeric(1)));
        assert_eq!(pdf.next(), Some(Token::Name("Kids")));
        assert_eq!(pdf.next(), Some(Token::ArrayBegin));
        assert_eq!(pdf.next(), Some(Token::IndirectRef(3, 0)));
        assert_eq!(pdf.next(), Some(Token::ArrayEnd));
        assert_eq!(pdf.next(), Some(Token::DictEnd));
        assert_eq!(pdf.next(), Some(Token::ObjEnd));
    }
}
