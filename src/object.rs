use core::panic;
use std::slice::Iter;

use tokenizer::{CharacterSet, Delimiter, Token, PdfBytes};

mod tokenizer;

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
                CharacterSet::Regular(b'+' | b'-') => (),
                CharacterSet::Regular(
                    b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9',
                ) => numeric = numeric * 10 + char::from(*curr).to_digit(10).unwrap(),
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

impl From<&mut PdfBytes<'_>> for IndirectObject {
    // Read bytes b"1 0 R: to IndirectRef
    fn from(byte: &mut PdfBytes<'_>) -> Self {
        let obj_num = match byte.next() {
            Some(Token::Numeric(n)) => Numeric(n),
            Some(t) => panic!("Unable to read components of indirect object; found incorrect first token {t:?}"),
            _ => panic!("Unable to read first component of indirect object"),
        };
        let obj_gen = match byte.next() {
            Some(Token::Numeric(n)) => Numeric(n),
            Some(t) => panic!("Unable to read components of indirect object; found incorrect second token {t:?}"),
            _ => panic!("Unable to read second component of indirect object"),
        };
        let is_reference = match byte.next() {
            Some(Token::String(b"R")) => true,
            Some(Token::String(b"obj")) => false,
            Some(c) => {
                panic!("Incoherent character found in third component of indirect object: {c:?}")
            }
            None => panic!("Unable to read third component of indirect object"),
        };
        IndirectObject {
            obj_num,
            obj_gen,
            is_reference,
        }
    }
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
    pub info: Option<IndirectObject>,    // Information dictionary
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
pub struct Catalog {
    // version: Option<Name>, // The version of the PDF specification to which the document conforms (for example, 1.4)
    pages: Option<IndirectObject>, // The page tree node that is the root of the document’s page tree
}

impl From<&[u8]> for Catalog {
    fn from(bytes: &[u8]) -> Self {
        let mut pdf = PdfBytes::new(bytes);
        // Consume object header
        IndirectObject::from(&mut pdf);

        match pdf.next() {
            Some(Token::DictBegin) => (),
            Some(t) => panic!("Catalog should be a dictionnary; found {t:?}"),
            None => panic!("Catalog should be a dictionnary"),
        };

        let mut pages= None;

        while let Some(t) = pdf.next() {
            match t {
                Token::Name(b"Type") => assert_eq!(pdf.next(), Some(Token::Name(b"Catalog"))),
                Token::Name(b"Pages") => {
                    pages = Some(IndirectObject::from(&mut pdf));
                }
                Token::DictEnd => break,
                a => panic!("Unexpected key was found in dictionnary catalog {a:?}"),
            };
        }
        Catalog { pages }
    }
}

#[derive(Debug, PartialEq)]
pub struct Info<'a> {
    title: Option<&'a str>,
    author: Option<&'a str>,
    creator: Option<&'a str>,
    producer: Option<&'a str>,
    creation_date: Option<&'a str>,
    mod_date: Option<&'a str>,
}

impl<'a> From<&'a [u8]> for Info<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        let mut pdf = PdfBytes::new(bytes);

        // Consume object header
        IndirectObject::from(&mut pdf);

        match pdf.next() {
            Some(Token::DictBegin) => (),
            Some(t) => panic!("Info should be a dictionnary; found {t:?}"),
            None => panic!("Info should be a dictionnary"),
        };

        let mut title= None;
        let mut author= None;
        let mut creator= None;
        let mut producer= None;
        let mut creation_date= None;
        let mut mod_date= None;

        while let Some(t) = pdf.next() {
            match t {
                Token::Name(b"Title") => match pdf.next() {
                    Some(Token::LitteralString(s)) => title = std::str::from_utf8(s).ok(),
                    Some(t) => panic!("Title should be a string; found {t:?}"),
                    _ => panic!("Title should be a string"),
                },
                Token::Name(b"Author") => match pdf.next() {
                    Some(Token::LitteralString(s)) => author = std::str::from_utf8(s).ok(),
                    _ => panic!("Author should be a string"),
                },
                Token::Name(b"Creator") => match pdf.next() {
                    Some(Token::LitteralString(s)) => creator = std::str::from_utf8(s).ok(),
                    _ => panic!("Creator should be a string"),
                },
                Token::Name(b"Producer") => match pdf.next() {
                    Some(Token::LitteralString(s)) => producer = std::str::from_utf8(s).ok(),
                    Some(t) => panic!("Producer should be a string; found {t:?}"),
                    _ => panic!("Producer should be a string"),
                },
                Token::Name(b"CreationDate") => match pdf.next() {
                    Some(Token::LitteralString(s)) => creation_date = std::str::from_utf8(s).ok(),
                    _ => panic!("CreationDate should be a string"),
                },
                Token::Name(b"ModDate") => match pdf.next() {
                    Some(Token::LitteralString(s)) => mod_date = std::str::from_utf8(s).ok(),
                    _ => panic!("Modification date should be a string"),
                },
                Token::Name(b"PTEX.Fullbanner") => {pdf.next();},
                Token::Name(n) => println!("Key {:?} is not implemented", std::str::from_utf8(n)),
                Token::DictEnd => break,
                t => panic!("Unexpected key was found in info dictionnary {t:?}"),
            };
        }
        Info { title, author, creator, producer, creation_date, mod_date }
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

    #[test]
    fn test_catalog() {
        let catalog = Catalog::from(b"1 0 obj  % entry point\n    <<\n      /Type /Catalog\n      /Pages 2 0 R\n    >>\n    endobj".as_slice());
        assert_eq!(
            catalog,
            Catalog {
                pages: Some(IndirectObject {
                    obj_num: Numeric(2),
                    obj_gen: Numeric(0),
                    is_reference: true
                })
            }
        )
    }

    #[test]
    fn test_info_dict_1() {
        let info_object = b"1 0 obj\n<< /Title (sample) /Author (Philip Hutchison) /Creator (Pages) /Producer (Mac OS X 10.5.4 Quartz PDFContext)\n/CreationDate (D:20080701052447Z00'00') /ModDate (D:20080701052447Z00'00')\n>>\nendobj";
        let info = Info::from(info_object.as_slice());
        assert_eq!(info, Info {
            title: Some("sample"),
            author: Some("Philip Hutchison"),
            creator: Some("Pages"),
            producer: Some("Mac OS X 10.5.4 Quartz PDFContext"),
            creation_date: Some("D:20080701052447Z00'00'"),
            mod_date: Some("D:20080701052447Z00'00'")
        });
    }
}
