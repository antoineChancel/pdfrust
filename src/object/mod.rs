// PDF basic objects
use std::collections::HashMap;
use tokenizer::{Token, Tokenizer};

mod tokenizer;

pub type Name = String;
pub type IndirectObject = (u32, u32);
pub type Numeric = u32;
pub type Array = Vec<Object>;
pub type Dictionary = HashMap<Name, Object>;

#[derive(Debug, PartialEq, Clone)]
pub enum Object {
    Dictionary(Dictionary),
    Stream(Dictionary, Vec<u8>),
    Array(Array),
    Name(Name),
    String(String),
    Numeric(Numeric),
    Ref(IndirectObject),
}

impl TryFrom<&mut Tokenizer<'_>> for Array {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Tokenizer<'_>) -> Result<Self, Self::Error> {
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

impl TryFrom<&mut Tokenizer<'_>> for Dictionary {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Tokenizer<'_>) -> Result<Self, Self::Error> {
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
impl<'a> TryFrom<&mut Tokenizer<'_>> for Object {
    type Error = &'static str;

    fn try_from(tokenizer: &mut Tokenizer<'_>) -> Result<Self, Self::Error> {
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

// attempt to create object from pdf bytes
impl TryFrom<&[u8]> for Object {
    type Error = &'static str;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let mut tokenizer = Tokenizer::new(bytes);
        Self::try_from(&mut tokenizer)
    }
}

// conversion of bare pdf token to object
impl<'a> TryFrom<Token<'a>> for Object {
    type Error = &'static str;

    fn try_from(token: Token) -> Result<Self, Self::Error> {
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
            Token::IndirectRef(obj, gen) => Ok(Object::Ref((obj, gen))),
            t => panic!("Unexpected token found in object{t:?}"),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_dictionnary_0() {
        let mut bytes =
            Tokenizer::new(b"/Title (sample) /Author (Philip Hutchison) /Creator (Pages) >>");
        let dict = Dictionary::try_from(&mut bytes).unwrap();
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
        let mut bytes =
            Tokenizer::new(b"1 0 obj  % entry point\n<<\n  /Type /Catalog\n\n>>\nendobj");
        match Object::try_from(&mut bytes) {
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
        let mut bytes = Tokenizer::new(b"2 0 obj\n<<\n  /Type /Pages\n  /MediaBox [ 0 0 200 200 ]\n  /Count 1\n  /Kids [ 3 0 R ]\n>>\nendobj");
        match Object::try_from(&mut bytes) {
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
                    Some(&Object::Array(vec![Object::Ref((3, 0))]))
                )
            }
            Ok(_) => todo!(),
            Err(_) => todo!(),
        }
    }

    #[test]
    fn test_object_page() {
        let mut bytes = Tokenizer::new(b"3 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /Resources <<\n    /Font <<\n      /F1 4 0 R \n    >>\n  >>\n  /Contents 5 0 R\n>>\nendobj");
        match Object::try_from(&mut bytes) {
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
