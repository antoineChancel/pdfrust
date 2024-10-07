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

    pub fn is_eol(&self) -> bool {
        match self {
            Self::LineFeed | Self::CarriageReturn => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum Delimiter {
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
    Regular { char: u8 },
    Delimiter { char: u8, value: Delimiter },
    WhiteSpace { char: u8, value: WhiteSpace },
}

impl CharacterSet {
    pub fn new(char: &u8) -> CharacterSet {
        match char {
            0 | 9 | 10 | 12 | 13 | 32 => CharacterSet::WhiteSpace {
                char: *char,
                value: WhiteSpace::new(*char),
            },
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' => {
                CharacterSet::Delimiter {
                    char: *char,
                    value: Delimiter::new(*char),
                }
            }
            _ => CharacterSet::Regular { char: *char },
        }
    }
}

#[derive(Debug, PartialEq)]
struct Name {
    value: String,
}

impl From<&[u8]> for Name {
    fn from(value: &[u8]) -> Self {
        let mut c = value.iter();
        // Name object starts with regular character /'
        match CharacterSet::new(c.next().unwrap()) {
            CharacterSet::Delimiter {
                char,
                value: Delimiter::Name,
            } => (),
            _ => panic!("Pdf name object should start with a name delimiter"),
        }
        let mut name = String::new();
        loop {
            let curr = match c.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::new(curr) {
                CharacterSet::Regular { char } => name.push(char::from(char)),
                _ => break,
            }
        }
        Name { value: name }
    }
}

#[derive(PartialEq, Debug)]
struct Numeric {
    value: u32,
}

impl From<&[u8]> for Numeric {
    fn from(value: &[u8]) -> Self {
        let mut c = value.iter();
        let mut value: u32 = 0;
        loop {
            let curr = match c.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::new(curr) {
                CharacterSet::Regular { char: b'+' | b'-' } => (),
                CharacterSet::Regular {
                    char: b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9',
                } => value = value * 10 + char::from(*curr).to_digit(10).unwrap(),
                _ => break,
            }
        }
        Self { value }
    }
}

struct IndirectObject {
    obj_num: Numeric,
    obj_gen: Numeric,
    is_reference: bool,
}

impl From<&[u8]> for IndirectRef {
    // Read bytes b"1 0 R: to IndirectRef
    fn from(bytes: &[u8]) -> Self {
        let c = bytes.iter();
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn read_name_object_from_u8() {
        let entry_sample = b"/Type /Font".as_slice();
        assert_eq!(
            Name::from(entry_sample),
            Name {
                value: String::from("Type")
            }
        );
    }

    #[test]
    fn read_name_object_from_u8_2() {
        let entry_sample = b"/Root".as_slice();
        assert_eq!(
            Name::from(entry_sample),
            Name {
                value: String::from("Root")
            }
        );
    }

    #[test]
    fn read_numeric_object() {
        let entry_sample = b"6".as_slice();
        assert_eq!(Numeric::from(entry_sample), Numeric { value: 6 });
    }

    #[test]
    fn read_numeric_object_with_sign() {
        let entry_sample = b"+54".as_slice();
        assert_eq!(Numeric::from(entry_sample), Numeric { value: 54 });
    }
}
