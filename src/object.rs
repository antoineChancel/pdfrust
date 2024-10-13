use core::panic;
use std::slice::Iter;

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
    Regular { char: u8 },
    Delimiter { char: u8, value: Delimiter },
    WhiteSpace { char: u8, value: WhiteSpace },
}

impl From<&u8> for CharacterSet {
    fn from(char: &u8) -> CharacterSet {
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

impl TryFrom<&mut Iter<'_, u8>> for Name {
    type Error = &'static str;

    fn try_from(value: &mut Iter<'_, u8>) -> Result<Self, Self::Error> {
        // Name object starts with regular character /'
        loop {
            match CharacterSet::from(value.next().unwrap()) {
                // Absorb eventual whitespaces before name
                CharacterSet::WhiteSpace { char: _, value: _ } => (),
                CharacterSet::Delimiter {
                    char: b'/',
                    value: Delimiter::Name,
                } => break,
                _ => return Err("Pdf name object should start with a name delimiter"),
            }
        }
        let mut name = String::new();
        loop {
            let curr = match value.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::from(curr) {
                CharacterSet::Regular { char } => name.push(char::from(char)),
                _ => break,
            }
        }
        Ok(Name { value: name })
    }
}

impl From<&[u8]> for Name {
    fn from(value: &[u8]) -> Self {
        let mut c = value.iter();
        // Name object starts with regular character /'
        match CharacterSet::from(c.next().unwrap()) {
            CharacterSet::Delimiter {
                char: _,
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
            match CharacterSet::from(curr) {
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

impl TryFrom<&mut Iter<'_, u8>> for Numeric {
    type Error = &'static str;

    fn try_from(value: &mut Iter<'_, u8>) -> Result<Self, Self::Error> {
        let mut numeric: u32 = 0;
        loop {
            let curr = match value.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::from(curr) {
                CharacterSet::Regular { char: b'+' | b'-' } => (),
                CharacterSet::Regular {
                    char: b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9',
                } => numeric = numeric * 10 + char::from(*curr).to_digit(10).unwrap(),
                _ => break,
            }
        }
        Ok(Self { value: numeric })
    }
}

#[derive(Debug, PartialEq)]
pub struct IndirectObject {
    obj_num: Numeric,
    obj_gen: Numeric,
    is_reference: bool,
}

impl From<&mut Iter<'_, u8>> for IndirectObject {
    // Read bytes b"1 0 R: to IndirectRef
    fn from(byte: &mut Iter<'_, u8>) -> Self {
        let obj_num = Numeric::try_from(&mut *byte).unwrap();
        let obj_gen = Numeric::try_from(&mut *byte).unwrap();
        let is_reference = match byte.next() {
            Some(b'R') => true,
            Some(b'o') => {
                byte.next().unwrap();
                byte.next().unwrap();
                false
            }
            Some(c) => {
                panic!("Incoherent character found in third component of indirect object: {c}")
            }
            None => panic!("Unable to read third component of indirect object"),
        };
        byte.next(); // TODO: check whitespace
        IndirectObject {
            obj_num,
            obj_gen,
            is_reference,
        }
    }
}

// extract trailer dictionnary
#[derive(Debug, PartialEq)]
pub struct Trailer {
    size: Numeric,
    prev: Option<Numeric>,
    root: IndirectObject,            // Catalogue dictionnary
    encrypt: Option<IndirectObject>, // Encryption dictionnary
    info: Option<IndirectObject>,    // Information dictionary
    id: Option<Vec<String>>,         // An array of two byte-strings constituting a file identifier
}

impl From<&[u8]> for Trailer {
    fn from(bytes: &[u8]) -> Self {
        let mut size = Numeric { value: 9999 };
        let mut root = IndirectObject {
            obj_num: Numeric { value: 0 },
            obj_gen: Numeric { value: 0 },
            is_reference: true,
        };
        let mut info = None;
        let id = None;
        let mut prev = None;
        let mut encrypt = None;

        let mut iter = bytes.iter();
        assert_eq!(*iter.next().unwrap(), b'<');
        assert_eq!(*iter.next().unwrap(), b'<');

        while let Ok(name) = Name::try_from(&mut iter) {
            match name.value.as_str() {
                "Size" => size = Numeric::try_from(&mut iter).unwrap(),
                "Root" => root = IndirectObject::try_from(&mut iter).unwrap(),
                "Info" => info = IndirectObject::try_from(&mut iter).ok(),
                "Prev" => prev = Numeric::try_from(&mut iter).ok(),
                "Encrypt" => encrypt = IndirectObject::try_from(&mut iter).ok(),
                "ID" => (), //id = Array::try_from(iter).ok(),
                a => panic!("Unexpected key was found in trailer {a}"),
            };
        }
        Trailer {
            size,
            prev,
            root,
            encrypt,
            info,
            id,
        }
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
        assert_eq!(
            Numeric::try_from(&mut entry_sample),
            Ok(Numeric { value: 6 })
        );
    }

    #[test]
    fn read_numeric_object_with_sign() {
        let mut entry_sample = b"+54".iter();
        assert_eq!(
            Numeric::try_from(&mut entry_sample),
            Ok(Numeric { value: 54 })
        );
    }

    #[test]
    fn read_indirect_object_ref() {
        let mut object_ref_sample = b"1 0 R".iter();
        assert_eq!(
            IndirectObject::from(&mut object_ref_sample),
            IndirectObject {
                obj_num: Numeric { value: 1 },
                obj_gen: Numeric { value: 0 },
                is_reference: true,
            }
        );
    }

    #[test]
    fn read_trailer_multi_lines() {
        let dict = b"<<\n  /Size 6\n  /Root 1 0 R\n>>".as_slice();
        assert_eq!(
            Trailer::from(dict),
            Trailer {
                size: Numeric { value: 6 },
                root: IndirectObject {
                    obj_num: Numeric { value: 1 },
                    obj_gen: Numeric { value: 0 },
                    is_reference: true
                },
                info: None,
                prev: None,
                encrypt: None,
                id: None
            }
        );
    }

    #[test]
    fn read_trailer_from_one_line() {
        let dict =
            b"<< /Size 26 /Root 13 0 R /Info 1 0 R /ID [ <4e949515aaf132498f650e7bde6cdc2f>\n<4e949515aaf132498f650e7bde6cdc2f> ] >>"
                .as_slice();
        assert_eq!(
            Trailer::from(dict),
            Trailer {
                size: Numeric { value: 26 },
                root: IndirectObject {
                    obj_num: Numeric { value: 13 },
                    obj_gen: Numeric { value: 0 },
                    is_reference: true
                },
                info: Some(IndirectObject {
                    obj_num: Numeric { value: 1 },
                    obj_gen: Numeric { value: 0 },
                    is_reference: true
                }),
                prev: None,
                encrypt: None,
                id: None
            }
        );
    }
}
