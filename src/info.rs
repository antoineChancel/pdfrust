use crate::xref::XRef;

use super::object::{Dictionary, Object};
use std::{fmt::Display, rc::Rc};

#[derive(Debug, PartialEq)]
pub struct Info {
    title: Option<String>,
    author: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<String>, // TODO: convert to DateTime
    mod_date: Option<String>,      // TODO: convert to DateTime
}

impl Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Title: {:?}\nAuthor: {:?}\nCreator: {:?}\nProducer: {:?}\nCreationDate: {:?}\nModDate: {:?}",
            self.title, self.author, self.creator, self.producer, self.creation_date, self.mod_date)
    }
}

impl Info {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: Rc<XRef>) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary<'_>> for Info {
    fn from(value: Dictionary) -> Self {
        Info {
            title: match value.get("Title") {
                Some(Object::String(s)) => Some(String::from(s)),
                None => None,
                _ => panic!("Title should be a string"),
            },
            author: match value.get("Author") {
                Some(Object::String(s)) => Some(String::from(s)),
                None => None,
                _ => panic!("Author should be a string"),
            },
            creator: match value.get("Creator") {
                Some(Object::String(s)) => Some(String::from(s)),
                Some(Object::HexString(s)) => Some(match std::str::from_utf8(s) {
                    Ok(s) => String::from(s),
                    Err(..) => String::new(), // in case of unable to read hexstring
                }),
                None => None,
                _ => panic!("Creator should be a string"),
            },
            producer: match value.get("Producer") {
                Some(Object::String(s)) => Some(String::from(s)),
                Some(Object::HexString(s)) => Some(match std::str::from_utf8(s) {
                    Ok(s) => String::from(s),
                    Err(..) => String::new(), // in case of unable to read hexstring
                }),
                None => None,
                _ => panic!("Producer should be a string"),
            },
            creation_date: match value.get("CreationDate") {
                Some(Object::String(s)) => Some(String::from(s)),
                None => None,
                _ => panic!("CreationDate should be a string"),
            },
            mod_date: match value.get("ModDate") {
                Some(Object::String(s)) => Some(String::from(s)),
                None => None,
                _ => panic!("ModDate should be a string"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::xref::XRefTable;

    use super::*;

    #[test]
    fn test_info_dict_1() {
        let bytes = b"1 0 obj\n<< /Title (sample) /Author (Philip Hutchison) /Creator (Pages) /Producer (Mac OS X 10.5.4 Quartz PDFContext)\n/CreationDate (D:20080701052447Z00'00') /ModDate (D:20080701052447Z00'00')\n>>\nendobj";
        let xref = Rc::new(XRef::XRefTable(XRefTable::default()));
        let info = Info::new(bytes.as_slice(), 0, xref);
        assert_eq!(
            info,
            Info {
                title: Some(String::from("sample")),
                author: Some(String::from("Philip Hutchison")),
                creator: Some(String::from("Pages")),
                producer: Some(String::from("Mac OS X 10.5.4 Quartz PDFContext")),
                creation_date: Some(String::from("D:20080701052447Z00'00'")),
                mod_date: Some(String::from("D:20080701052447Z00'00'"))
            }
        );
    }
}
