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
pub enum PdfVersion {
    V1_3,
    V1_4,
    V1_7,
}

pub fn pdf_version(s: &[u8]) -> PdfVersion {
    match &s[s.len() - 3..] {
        b"1.7" => PdfVersion::V1_7,
        b"1.4" => PdfVersion::V1_4,
        b"1.3" => PdfVersion::V1_3,
        _ => panic!("Pdf version not supported"),
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
        // Name object starts with regular character /
        match CharacterSet::new(c.next().unwrap()) {
            CharacterSet::Delimiter { char, value: Delimiter::Name } => (), 
            _ => panic!("Pdf name object should start with a name delimiter"),
        }
        let mut name = String::new();
        loop {
            match CharacterSet::new(c.next().unwrap()) {
                CharacterSet::Regular { char } => name.push(char::from(char)),
                _ => break,
            }
        }
        Name { value: name }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn read_name_object_from_u8() {
        let entry_sample = b"/Type /Font".as_slice();
        assert_eq!(Name::from(entry_sample), Name{ value: String::from("Type")});
    }
}
