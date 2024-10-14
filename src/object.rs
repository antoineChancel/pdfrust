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

#[derive(Debug, PartialEq)]
enum Token<'a> {
    Numeric(u32),
    String(&'a [u8]),
    Name(Name),
    Comment(&'a [u8]),
    Stream(&'a [u8]),
}

struct PdfBytes<'a> {
    bytes: &'a [u8],
    curr_idx: usize,
}

impl<'a> PdfBytes<'a> {
    fn new(bytes: &'a [u8]) -> PdfBytes<'a> {
        PdfBytes { bytes, curr_idx: 0 }
    }
}

impl<'a> Iterator for PdfBytes<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut token = None;
        loop {
            match CharacterSet::from(&self.bytes[self.curr_idx]) {
                CharacterSet::Delimiter(v) => match v {
                    Delimiter::Comment => {
                        // read all characters until a line feed or cariage return is met
                        let begin = self.curr_idx + 1;
                        loop {
                            self.curr_idx += 1;
                            // end of stream
                            if self.curr_idx >= self.bytes.len() {
                                break
                            }
                            match CharacterSet::from(&self.bytes[self.curr_idx]) {
                                CharacterSet::WhiteSpace(WhiteSpace::CarriageReturn) => break,
                                CharacterSet::WhiteSpace(WhiteSpace::LineFeed) => break,
                                _ => ()
                            }
                        }
                        token = Some(Token::Comment(&self.bytes[begin..self.curr_idx]));
                        break;
                    }
                    Delimiter::String | Delimiter::Array | Delimiter::Name => (),
                },
                // read regular string
                CharacterSet::Regular(_) => {
                    let begin = self.curr_idx;
                    let mut is_numeric = true;
                    loop {
                        self.curr_idx += 1;
                        match CharacterSet::from(&self.bytes[self.curr_idx]) {
                            CharacterSet::Regular(b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9') => (),
                            CharacterSet::Regular(_) => is_numeric = false,
                            _ => break
                        }
                    }
                    if is_numeric {
                        let mut numeric = 0;
                        for c in &self.bytes[begin..self.curr_idx] {
                            numeric = numeric * 10 + char::from(*c).to_digit(10).unwrap()
                        }
                        token = Some(Token::Numeric(numeric));
                    } else {
                        token = Some(Token::String(&self.bytes[begin..self.curr_idx]));
                    }
                    break;
                }
                // absorb whitespaces before a new token is met
                CharacterSet::WhiteSpace(_) => self.curr_idx += 1,
            }
        }
        token
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

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Name {
    value: String,
}

impl TryFrom<&mut Iter<'_, u8>> for Name {
    type Error = &'static str;

    fn try_from(value: &mut Iter<'_, u8>) -> Result<Self, Self::Error> {
        // Name object starts with regular character /'
        loop {
            match CharacterSet::from(value.next().unwrap()) {
                // Absorb eventual whitespaces before name
                CharacterSet::WhiteSpace(_) => (),
                CharacterSet::Delimiter(Delimiter::Name) => break,
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
                CharacterSet::Regular(c) => name.push(char::from(c)),
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
            CharacterSet::Delimiter(Delimiter::Name) => (),
            _ => panic!("Pdf name object should start with a name delimiter"),
        }
        let mut name = String::new();
        loop {
            let curr = match c.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::from(curr) {
                CharacterSet::Regular(c) => name.push(char::from(c)),
                _ => break,
            }
        }
        Name { value: name }
    }
}

#[derive(PartialEq, Eq, Debug, Hash)]
pub struct Numeric(pub u32);

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
                CharacterSet::Regular (b'+' | b'-') => (),
                CharacterSet::Regular (b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9') => numeric = numeric * 10 + char::from(*curr).to_digit(10).unwrap(),
                _ => break,
            }
        }
        Ok(Self(numeric))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct IndirectObject {
    pub obj_num: Numeric,
    pub obj_gen: Numeric,
    pub is_reference: bool,
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
    pub root: IndirectObject,        // Catalogue dictionnary
    encrypt: Option<IndirectObject>, // Encryption dictionnary
    info: Option<IndirectObject>,    // Information dictionary
    id: Option<Vec<String>>,         // An array of two byte-strings constituting a file identifier
}

impl From<&[u8]> for Trailer {
    fn from(bytes: &[u8]) -> Self {
        let mut size = Numeric(9999);
        let mut root = IndirectObject {
            obj_num: Numeric(0),
            obj_gen: Numeric(0),
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

#[derive(Debug, PartialEq)]
// Defined in page 139;  commented is to be implemented
struct Catalog {
    // version: Option<Name>, // The version of the PDF specification to which the document conforms (for example, 1.4)
    pages: IndirectObject, // The page tree node that is the root of the document’s page tree
                           // page_labels: Option<IndirectObject>,
                           // names: Option<IndirectObject>,
                           // dests: Option<IndirectObject>,
                           // viewer_preferences: Option<IndirectObject>,
                           // page_layout: Option<Name>,
                           // page_mode: Option<Name>,
                           // outlines: Option<IndirectObject>,
                           // threads: Vec<String>,
                           // open_action: Option<IndirectObject>,
                           // aa: Option<IndirectObject>,
                           // uri: Option<IndirectObject>,
                           // acro_form: Option<IndirectObject>
}

impl From<&[u8]> for Catalog {
    fn from(bytes: &[u8]) -> Self {
        let mut pages = IndirectObject {
            obj_num: Numeric(0),
            obj_gen: Numeric(0),
            is_reference: true,
        };

        let mut iter = bytes.iter();

        // Read object header
        IndirectObject::try_from(&mut iter).unwrap();

        assert_eq!(*iter.next().unwrap(), b'<');
        assert_eq!(*iter.next().unwrap(), b'<');

        while let Ok(name) = Name::try_from(&mut iter) {
            match name.value.as_str() {
                "Pages" => pages = IndirectObject::try_from(&mut iter).unwrap(),
                "Type" => {
                    assert_eq!(
                        Name::try_from(&mut iter).unwrap(),
                        Name {
                            value: String::from("Catalog")
                        }
                    )
                }
                a => panic!("Unexpected key was found in catalog {a}"),
            };
        }
        Catalog { pages }
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
        assert_eq!(Numeric::try_from(&mut entry_sample), Ok(Numeric(6)));
    }

    #[test]
    fn read_numeric_object_with_sign() {
        let mut entry_sample = b"+54".iter();
        assert_eq!(Numeric::try_from(&mut entry_sample), Ok(Numeric(54)));
    }

    #[test]
    fn read_indirect_object_ref() {
        let mut object_ref_sample = b"1 0 R".iter();
        assert_eq!(
            IndirectObject::from(&mut object_ref_sample),
            IndirectObject {
                obj_num: Numeric(1),
                obj_gen: Numeric(0),
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
                size: Numeric(6),
                root: IndirectObject {
                    obj_num: Numeric(1),
                    obj_gen: Numeric(0),
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
                size: Numeric(26),
                root: IndirectObject {
                    obj_num: Numeric(13),
                    obj_gen: Numeric(0),
                    is_reference: true
                },
                info: Some(IndirectObject {
                    obj_num: Numeric(1),
                    obj_gen: Numeric(0),
                    is_reference: true
                }),
                prev: None,
                encrypt: None,
                id: None
            }
        );
    }

    //     #[test]
    //     fn test_catalog() {
    //         let catalog = Catalog::from(b"1 0 obj  % entry point
    // <<
    //   /Type /Catalog
    //   /Pages 2 0 R
    // >>
    // endobj".as_slice());
    //     assert_eq!(catalog, Catalog {
    //         pages: IndirectObject {
    //             obj_num: Numeric(2),
    //             obj_gen: Numeric(0),
    //             is_reference: true
    //         }
    //     })
    //     }

    #[test]
    fn test_pdfbytes_iterator() {
        let mut pdf = PdfBytes::new(b"%PDF-1.7\n\n1 0 obj  % entry point");
        assert_eq!(pdf.next(), Some(Token::Comment(b"PDF-1.7")));
        assert_eq!(pdf.next(), Some(Token::Numeric(1)));
        assert_eq!(pdf.next(), Some(Token::Numeric(0)));
        assert_eq!(pdf.next(), Some(Token::String(b"obj")));
        assert_eq!(pdf.next(), Some(Token::Comment(b" entry point")));
    }
}
