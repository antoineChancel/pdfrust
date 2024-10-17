use super::object::{Dictionary, Object};

#[derive(Debug, PartialEq)]
pub struct Info {
    title: Option<String>,
    author: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<String>,
    mod_date: Option<String>,
}

impl From<&[u8]> for Info {
    fn from(bytes: &[u8]) -> Self {
        match Object::try_from(bytes).unwrap() {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary> for Info {
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
                None => None,
                _ => panic!("Creator should be a string"),
            },
            producer: match value.get("Producer") {
                Some(Object::String(s)) => Some(String::from(s)),
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
    use super::*;

    #[test]
    fn test_info_dict_1() {
        let info_object = b"1 0 obj\n<< /Title (sample) /Author (Philip Hutchison) /Creator (Pages) /Producer (Mac OS X 10.5.4 Quartz PDFContext)\n/CreationDate (D:20080701052447Z00'00') /ModDate (D:20080701052447Z00'00')\n>>\nendobj";
        let info = Info::from(info_object.as_slice());
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
