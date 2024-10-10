use core::panic;
use std::{collections::HashMap, slice::Iter};

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

#[derive(Debug, PartialEq, Eq, Hash)]
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

impl From<&mut Iter<'_, u8>> for Numeric {
    fn from(byte: &mut Iter<'_, u8>) -> Self {
        let mut value: u32 = 0;
        loop {
            let curr = match byte.next() {
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

#[derive(Debug, PartialEq)]
struct IndirectObject {
    obj_num: Numeric,
    obj_gen: Numeric,
    is_reference: bool,
}

impl From<&mut Iter<'_, u8>> for IndirectObject {
    // Read bytes b"1 0 R: to IndirectRef
    fn from(byte: &mut Iter<'_, u8>) -> Self {
        let obj_num = Numeric::from(&mut *byte);
        let obj_gen = Numeric::from(byte);
        let is_reference = true;
        IndirectObject {
            obj_num, obj_gen, is_reference
        }
    }
}

enum DictValue {
    Name(Name),
    Dict(Dict),
    Numeric(Numeric),
    String(String),
    Bool(bool)
}

struct Dict {
    value: HashMap<Name, DictValue>
}

impl Dict {
    fn read_entry(line: &str) -> (Name, DictValue) {
        (Name {
            value: String::from("TBD")
        }, DictValue::Name(Name{value: String::from("antoine")}))
    }
}

impl From<&str> for Dict {

    fn from(bytes: &str) -> Self {
        let mut lines = bytes.lines();
        // First line is <<
        match lines.next() {
            Some("<<") => (),
            Some(l) => panic!("PDF dictionnary should start with '<<' found {l}"),
            _ => panic!("PDF dictionnary missing line")
        }
        let mut value = HashMap::new();
        loop {
            match lines.next() {
                Some(">>") => break,
                Some(l) => {
                    let (k, v) = Dict::read_entry(l);
                    value.insert(k, v);
                },
                None => panic!("PDF dictionnary parser reached end of stream without finding a >>")
            };
        }
        Dict { value }
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
        let mut entry_sample = b"6".iter();
        assert_eq!(Numeric::from(&mut entry_sample), Numeric { value: 6 });
    }

    #[test]
    fn read_numeric_object_with_sign() {
        let mut entry_sample = b"+54".iter();
        assert_eq!(Numeric::from(&mut entry_sample), Numeric { value: 54 });
    }

    #[test]
    fn read_indirect_object_ref() {
        let mut object_ref_sample = b"1 0 R".iter();
        assert_eq!(IndirectObject::from(&mut object_ref_sample), IndirectObject {
            obj_num: Numeric { value: 1 },
            obj_gen: Numeric { value: 0 },
            is_reference: true,
        });
    }
}
