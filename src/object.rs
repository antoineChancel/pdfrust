// PDF basic objects
pub use crate::tokenizer::{Lemmatizer, Token};
use std::collections::HashMap;

use crate::{algebra::Number, tokenizer::Tokenizer};

pub type Name = String;
pub type IndirectObject = (i32, i32);
pub type Array = Vec<Object>;
pub type Dictionary = HashMap<Name, Object>;

#[derive(Debug, PartialEq, Clone)]
pub struct Stream {
    pub header: Dictionary,
    pub bytes: Vec<u8>,
}

impl Stream {
    fn new(header: Dictionary, bytes: Vec<u8>) -> Self {
        Stream { header, bytes }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Object {
    Dictionary(Dictionary),
    Stream(Stream),
    Array(Array),
    Name(Name),
    String(String),
    HexString(Vec<u8>),
    Numeric(Number),
    Ref(IndirectObject),
}

impl<'a> TryFrom<&mut Lemmatizer<'a>> for Array {
    type Error = &'static str;

    fn try_from(lemmatizer: &mut Lemmatizer<'a>) -> Result<Self, Self::Error> {
        let mut array = Array::new();
        for t in lemmatizer.by_ref() {
            match t {
                Token::ArrayEnd => break,
                _ => array.push(Object::try_from(t).unwrap()),
            }
        }
        Ok(array)
    }
}

impl<'a> TryFrom<&mut Lemmatizer<'a>> for Dictionary {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Lemmatizer<'a>) -> Result<Self, Self::Error> {
        let mut dict = Dictionary::new();
        while let Some(t) = tokenizer.next() {
            match t {
                Token::Name(name) => {
                    let key = name;
                    let value = match tokenizer.next() {
                        Some(Token::DictBegin) => {
                            Object::Dictionary(Dictionary::try_from(&mut *tokenizer).unwrap())
                        }
                        Some(Token::ArrayBegin) => {
                            Object::Array(Array::try_from(&mut *tokenizer).unwrap())
                        }
                        Some(Token::LitteralString(s)) => {
                            Object::String(String::from(std::str::from_utf8(&s).unwrap()))
                        }
                        Some(Token::String(s)) => {
                            Object::Name(String::from(std::str::from_utf8(&s).unwrap()))
                        }
                        Some(Token::HexString(s)) => Object::HexString(s),
                        Some(Token::Name(n)) => Object::Name(n),
                        Some(Token::Numeric(n)) => Object::Numeric(n),
                        Some(Token::IndirectRef(obj, gen)) => Object::Ref((obj, gen)),
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
impl<'a> TryFrom<&mut Lemmatizer<'a>> for Object {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Lemmatizer<'a>) -> Result<Self, Self::Error> {
        let object;
        'start: loop {
            match tokenizer.next() {
                Some(Token::ObjBegin) => continue 'start,
                Some(Token::DictBegin) => {
                    let dict = Dictionary::try_from(&mut *tokenizer).unwrap();
                    // check if next token is stream
                    object = match tokenizer.next() {
                        // object stream
                        Some(Token::StreamBegin) => {
                            let length = match dict.get("Length") {
                                Some(Object::Numeric(Number::Integer(n))) => *n,
                                Some(Object::Numeric(Number::Real(_))) => {
                                    panic!("Real number found in stream length")
                                }
                                // follow reference to indirect object is required to get the length
                                // Some(Object::Ref((obj, gen))) => Ref::Ref(*obj, *gen),
                                _ => panic!("Stream dictionary should have a Length key, {dict:?}"),
                            };
                            // collect next n bytes from the stream
                            Object::Stream(Stream::new(dict, tokenizer.next_n(length as usize)))
                        }
                        _ => Object::Dictionary(dict),
                    };
                    break;
                }
                Some(Token::Numeric(n)) => {
                    object = Object::Numeric(n);
                    break;
                }
                Some(Token::String(s)) => panic!("{s:?}"),
                Some(Token::ArrayBegin) => {
                    object = Object::Array(Array::try_from(&mut *tokenizer).unwrap());
                    break;
                }
                Some(t) => panic!("Unexpected token found in object; found {:?}", t),
                None => panic!("Unexpected end of stream found in object"),
            };
        }
        Ok(object)
    }
}

impl Object {
    pub fn new(bytes: &[u8]) -> Self {
        Self::try_from(&mut Lemmatizer::new(bytes)).unwrap()
    }
}

impl<'a> From<&mut Tokenizer<'a>> for Object {
    fn from(value: &mut Tokenizer<'a>) -> Self {
        Self::try_from(&mut Lemmatizer::from(value.clone())).unwrap()
    }
}

// conversion of bare pdf token to object
impl TryFrom<Token> for Object {
    type Error = &'static str;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
        match token {
            Token::DictBegin => Ok(Object::Dictionary(Dictionary::new())),
            Token::ArrayBegin => Ok(Object::Array(Array::new())),
            // Token::IndirectObject => Ok(Object::Ref(IndirectObject::try_from(&mut tokenizer).unwrap())),
            Token::Name(n) => Ok(Object::Name(n)),
            Token::Numeric(n) => Ok(Object::Numeric(n)),
            Token::String(s) => Ok(Object::String(String::from(
                std::str::from_utf8(&s).unwrap(),
            ))),
            Token::LitteralString(s) => Ok(Object::String(String::from(
                std::str::from_utf8(&s).unwrap(),
            ))),
            Token::HexString(s) => Ok(Object::HexString(s)),
            Token::IndirectRef(obj, gen) => Ok(Object::Ref((obj, gen))),
            t => panic!("Unexpected token found in object{t:?}"),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::tokenizer::Lemmatizer;

    use super::*;

    #[test]
    fn test_dictionnary_0() {
        let mut t =
            Lemmatizer::new(b"/Title (sample) /Author (Philip Hutchison) /Creator (Pages) >>");
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
    fn test_object_trailer() {
        let bytes = b"<</Size 14/Root 12 0 R\n/Info 13 0 R\n/ID [ <6285DCD147BBD7C07D63844C37B01D23>\n<6285DCD147BBD7C07D63844C37B01D23> ]\n/DocChecksum /700D49F24CC4E7F9CC731421E1DAB422\n>>\nstartxref\n12125\n";
        let mut t = Lemmatizer::new(bytes);
        match Object::try_from(&mut t) {
            Ok(Object::Dictionary(d)) => {
                assert_eq!(
                    d.get(&String::from("Size")),
                    Some(&Object::Numeric(Number::Integer(14)))
                );
                assert_eq!(d.get(&String::from("Root")), Some(&Object::Ref((12, 0))));
                assert_eq!(d.get(&String::from("Info")), Some(&Object::Ref((13, 0))));
                assert_eq!(
                    d.get(&String::from("ID")),
                    Some(&Object::Array(vec![
                        Object::HexString(
                            [
                                98, 133, 220, 209, 71, 187, 215, 192, 125, 99, 132, 76, 55, 176,
                                29, 35
                            ]
                            .to_vec()
                        ),
                        Object::HexString(
                            [
                                98, 133, 220, 209, 71, 187, 215, 192, 125, 99, 132, 76, 55, 176,
                                29, 35
                            ]
                            .to_vec()
                        )
                    ]))
                );
                assert_eq!(
                    d.get(&String::from("DocChecksum")),
                    Some(&Object::Name(String::from(
                        "700D49F24CC4E7F9CC731421E1DAB422"
                    )))
                );
            }
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    #[test]
    fn test_object_catalog() {
        let mut t = Lemmatizer::new(b"1 0 obj  % entry point\n<<\n  /Type /Catalog\n\n>>\nendobj");
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
        let bytes = b"2 0 obj\n<<\n  /Type /Pages\n  /MediaBox [ 0 0 200 200 ]\n  /Count 1\n  /Kids [ 3 0 R ]\n>>\nendobj";
        let mut t = Lemmatizer::new(bytes);
        match Object::try_from(&mut t) {
            Ok(Object::Dictionary(d)) => {
                assert_eq!(
                    d.get(&String::from("Type")),
                    Some(&Object::Name(String::from("Pages")))
                );
                assert_eq!(
                    d.get(&String::from("MediaBox")),
                    Some(&Object::Array(vec![
                        Object::Numeric(Number::Integer(0)),
                        Object::Numeric(Number::Integer(0)),
                        Object::Numeric(Number::Integer(200)),
                        Object::Numeric(Number::Integer(200))
                    ]))
                );
                assert_eq!(
                    d.get(&String::from("Count")),
                    Some(&Object::Numeric(Number::Integer(1)))
                );
                assert_eq!(
                    d.get(&String::from("Kids")),
                    Some(&Object::Array(vec![Object::Ref((3, 0))]))
                )
            }
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    #[test]
    fn test_object_stream() {
        let bytes = b"4 0 obj\n<<\n  /Length 10\n>>\nstream\n1234567890\nendstream\nendobj";
        let mut t = Lemmatizer::new(bytes);
        match Object::try_from(&mut t) {
            Ok(Object::Stream(Stream {
                header: d,
                bytes: s,
            })) => {
                assert_eq!(
                    d.get(&String::from("Length")),
                    Some(&Object::Numeric(Number::Integer(10)))
                );
                assert_eq!(s, b"1234567890");
            }
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    #[test]
    fn test_object_page() {
        let bytes = b"3 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /Resources <<\n    /Font <<\n      /F1 4 0 R \n    >>\n  >>\n  /Contents 5 0 R\n>>\nendobj";
        let mut t = Lemmatizer::new(bytes);
        match Object::try_from(&mut t) {
            Ok(Object::Dictionary(d)) => {
                assert_eq!(
                    d.get(&String::from("Type")),
                    Some(&Object::Name(String::from("Page")))
                );
                assert_eq!(d.get(&String::from("Parent")), Some(&Object::Ref((2, 0))));
                assert_eq!(d.get(&String::from("Contents")), Some(&Object::Ref((5, 0))));
                match d.get(&String::from("Resources")) {
                    Some(Object::Dictionary(d)) => match d.get(&String::from("Font")) {
                        Some(Object::Dictionary(d)) => {
                            assert_eq!(d.get(&String::from("F1")), Some(&Object::Ref((4, 0))));
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
