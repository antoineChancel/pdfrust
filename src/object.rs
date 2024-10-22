// PDF basic objects
use crate::tokenizer::{Token, Tokenizer};
use std::collections::HashMap;

use crate::xref::XrefTable;

pub type Name = String;
pub type IndirectObject = (u32, u32);
pub type Numeric = u32;
pub type Array<'a> = Vec<Object<'a>>;
pub type Dictionary<'a> = HashMap<Name, Object<'a>>;

#[derive(Debug, PartialEq, Clone)]
pub enum Object<'a> {
    Dictionary(Dictionary<'a>),
    Stream(Dictionary<'a>, Vec<u8>),
    Array(Array<'a>),
    Name(Name),
    String(String),
    Numeric(Numeric),
    Ref(IndirectObject, &'a XrefTable, &'a [u8]),
}

impl<'a> TryFrom<&mut Tokenizer<'a>> for Array<'a> {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Tokenizer<'a>) -> Result<Self, Self::Error> {
        let mut array = Array::new();
        while let Some(t) = tokenizer.next() {
            match t {
                Token::ArrayEnd => break,
                _ => array.push(Object::try_from(t).unwrap()),
            }
        }
        Ok(array)
    }
}

impl<'a> TryFrom<&mut Tokenizer<'a>> for Dictionary<'a> {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Tokenizer<'a>) -> Result<Self, Self::Error> {
        let mut dict = Dictionary::new();
        while let Some(t) = tokenizer.next() {
            match t {
                Token::Name(name) => {
                    let key = String::from(name);
                    let value = match tokenizer.next() {
                        Some(Token::DictBegin) => {
                            Object::Dictionary(Dictionary::try_from(&mut *tokenizer).unwrap())
                        }
                        Some(Token::ArrayBegin) => {
                            Object::Array(Array::try_from(&mut *tokenizer).unwrap())
                        }
                        Some(Token::LitteralString(s)) => {
                            Object::String(String::from(std::str::from_utf8(s).unwrap()))
                        }
                        Some(Token::String(s)) => {
                            Object::Name(String::from(std::str::from_utf8(s).unwrap()))
                        }
                        Some(Token::HexString(s)) => {
                            Object::String(String::from(std::str::from_utf8(s).unwrap()))
                        }
                        Some(Token::Name(n)) => Object::Name(String::from(n)),
                        Some(Token::Numeric(n)) => Object::Numeric(n),
                        Some(Token::IndirectRef((obj, gen), xref, bytes)) => {
                            Object::Ref((obj, gen), xref, bytes)
                        }
                        Some(t) => panic!(
                            "Unexpected token found in dictionary value {token:?}",
                            token = t
                        ),
                        None => panic!("Unexpected end of stream found in dictionary value"),
                    };
                    dict.insert(key, value);
                }
                Token::DictEnd => break,
                t => panic!("Unexpected token found in dictionary key {t:?}"),
            }
        }
        Ok(dict)
    }
}

// object creation from tokenizer (pdf body)
impl<'a> TryFrom<&mut Tokenizer<'a>> for Object<'a> {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Tokenizer<'a>) -> Result<Self, Self::Error> {
        let object;
        'start: loop {
            match tokenizer.next() {
                Some(Token::ObjBegin) => continue 'start,
                Some(Token::DictBegin) => {
                    object = Object::Dictionary(Dictionary::try_from(&mut *tokenizer).unwrap());
                    break;
                }
                Some(t) => panic!("Unexpected token found in object; found {:?}", t),
                None => panic!("Unexpected end of stream found in object"),
            };
        }
        Ok(object)
    }
}

impl<'a> Object<'a> {
    pub fn new(bytes: &'a [u8], curr_idx: usize, xref: &'a XrefTable) -> Self {
        Self::try_from(&mut Tokenizer::new(bytes, curr_idx, xref)).unwrap()
    }
}

// conversion of bare pdf token to object
impl<'a> TryFrom<Token<'a>> for Object<'a> {
    type Error = &'static str;

    fn try_from(token: Token<'a>) -> Result<Self, Self::Error> {
        match token {
            Token::DictBegin => Ok(Object::Dictionary(Dictionary::new())),
            Token::ArrayBegin => Ok(Object::Array(Array::new())),
            // Token::IndirectObject => Ok(Object::Ref(IndirectObject::try_from(&mut tokenizer).unwrap())),
            Token::Name(n) => Ok(Object::Name(String::from(n))),
            Token::Numeric(n) => Ok(Object::Numeric(n)),
            Token::String(s) => Ok(Object::String(String::from(
                std::str::from_utf8(s).unwrap(),
            ))),
            Token::LitteralString(s) => Ok(Object::String(String::from(
                std::str::from_utf8(s).unwrap(),
            ))),
            Token::HexString(s) => Ok(Object::String(String::from(
                std::str::from_utf8(s).unwrap(),
            ))),
            Token::IndirectRef((obj, gen), xref, bytes) => Ok(Object::Ref((obj, gen), xref, bytes)),
            t => panic!("Unexpected token found in object{t:?}"),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::xref;

    use super::*;

    #[test]
    fn test_dictionnary_0() {
        let xref = &xref::XrefTable::new();
        let mut t = Tokenizer::new(
            b"/Title (sample) /Author (Philip Hutchison) /Creator (Pages) >>",
            0,
            &xref,
        );
        let dict = Dictionary::try_from(&mut t).unwrap();
        assert_eq!(
            dict.get(&String::from("Title")),
            Some(&Object::String("sample".to_string()))
        );
        assert_eq!(
            dict.get(&String::from("Author")),
            Some(&Object::String("Philip Hutchison".to_string()))
        );
        assert_eq!(
            dict.get(&String::from("Creator")),
            Some(&Object::String("Pages".to_string()))
        );
    }

    #[test]
    fn test_object_catalog() {
        let xref = &XrefTable::new();
        let mut t = Tokenizer::new(
            b"1 0 obj  % entry point\n<<\n  /Type /Catalog\n\n>>\nendobj",
            0,
            &xref,
        );
        match Object::try_from(&mut t) {
            Ok(Object::Dictionary(d)) => {
                assert_eq!(
                    d.get(&String::from("Type")),
                    Some(&Object::Name(String::from("Catalog")))
                );
            }
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    #[test]
    fn test_object_pages() {
        let xref = &XrefTable::new();
        let bytes = b"2 0 obj\n<<\n  /Type /Pages\n  /MediaBox [ 0 0 200 200 ]\n  /Count 1\n  /Kids [ 3 0 R ]\n>>\nendobj";
        let mut t = Tokenizer::new(bytes, 0, &xref);
        match Object::try_from(&mut t) {
            Ok(Object::Dictionary(d)) => {
                assert_eq!(
                    d.get(&String::from("Type")),
                    Some(&Object::Name(String::from("Pages")))
                );
                assert_eq!(
                    d.get(&String::from("MediaBox")),
                    Some(&Object::Array(vec![
                        Object::Numeric(0),
                        Object::Numeric(0),
                        Object::Numeric(200),
                        Object::Numeric(200)
                    ]))
                );
                assert_eq!(d.get(&String::from("Count")), Some(&Object::Numeric(1)));
                assert_eq!(
                    d.get(&String::from("Kids")),
                    Some(&Object::Array(vec![Object::Ref((3, 0), xref, bytes)]))
                )
            }
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    #[test]
    fn test_object_page() {
        let xref = &XrefTable::new();
        let bytes = b"3 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /Resources <<\n    /Font <<\n      /F1 4 0 R \n    >>\n  >>\n  /Contents 5 0 R\n>>\nendobj";
        let mut t = Tokenizer::new(bytes, 0, &xref);
        match Object::try_from(&mut t) {
            Ok(Object::Dictionary(d)) => {
                assert_eq!(
                    d.get(&String::from("Type")),
                    Some(&Object::Name(String::from("Page")))
                );
                assert_eq!(
                    d.get(&String::from("Parent")),
                    Some(&Object::Ref((2, 0), &xref, bytes))
                );
                assert_eq!(
                    d.get(&String::from("Contents")),
                    Some(&Object::Ref((5, 0), &xref, bytes))
                );
                match d.get(&String::from("Resources")) {
                    Some(Object::Dictionary(d)) => match d.get(&String::from("Font")) {
                        Some(Object::Dictionary(d)) => {
                            assert_eq!(
                                d.get(&String::from("F1")),
                                Some(&Object::Ref((4, 0), &xref, bytes))
                            );
                        }
                        _ => panic!("Resources should be a dictionnary"),
                    },
                    _ => panic!("Resources should be a dictionnary"),
                }
            }
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }
}
