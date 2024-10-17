use crate::object::{Array, Dictionary, IndirectObject, Numeric, Object};

// extract trailer dictionnary
#[derive(Debug, PartialEq)]
pub struct Trailer {
    size: Numeric,
    prev: Option<Numeric>,
    // Catalogue dictionnary or a reference to the root object of the page tree
    pub root: IndirectObject,
    // Encryption dictionnary
    encrypt: Option<IndirectObject>,
    // Information dictionary containing metadata
    pub info: Option<IndirectObject>,
    // An array of two byte-strings constituting a file identifier
    id: Option<Array>,
}

impl From<&[u8]> for Trailer {
    fn from(bytes: &[u8]) -> Self {
        let trailer = Object::try_from(bytes).unwrap();

        match trailer {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary> for Trailer {
    fn from(value: Dictionary) -> Self {
        Trailer {
            size: match value.get("Size").unwrap() {
                Object::Numeric(n) => *n,
                _ => panic!("Size should be a numeric"),
            },
            prev: match value.get("Prev") {
                Some(Object::Numeric(n)) => Some(*n),
                None => None,
                _ => panic!("Prev should be a numeric"),
            },
            root: match value.get("Root").unwrap() {
                Object::Ref((obj, gen)) => (*obj, *gen),
                _ => panic!("Root should be an indirect object"),
            },
            encrypt: match value.get("Encrypt") {
                Some(Object::Ref((obj, gen))) => Some((*obj, *gen)),
                None => None,
                _ => panic!("Encrypt should be an indirect object"),
            },
            info: match value.get("Info") {
                Some(Object::Ref((obj, gen))) => Some((*obj, *gen)),
                None => None,
                _ => panic!("Info should be an indirect object"),
            },
            id: match value.get("ID") {
                Some(Object::Array(arr)) => Some(arr.clone()),
                None => None,
                _ => panic!("ID should be an array"),
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_trailer_multi_lines() {
        let dict = b"<<\n  /Size 6\n  /Root 1 0 R\n>>".as_slice();
        assert_eq!(
            Trailer::from(dict),
            Trailer {
                size: 6,
                root: (1, 0),
                info: None,
                prev: None,
                encrypt: None,
                id: None
            }
        );
    }

    #[test]
    fn read_trailer_from_one_line() {
        // Array is not read correctly -> to fix
        let dict =
            b"<< /Size 26 /Root 13 0 R /Info 1 0 R /ID [ <4e949515aaf132498f650e7bde6cdc2f>\n<4e949515aaf132498f650e7bde6cdc2f> ] >>"
                .as_slice();
        assert_eq!(
            Trailer::from(dict),
            Trailer {
                size: 26,
                root: (13, 0),
                info: Some((1, 0)),
                prev: None,
                encrypt: None,
                id: Some(vec![
                    Object::String("4e949515aaf132498f650e7bde6cdc2f".to_string()),
                    Object::String("4e949515aaf132498f650e7bde6cdc2f".to_string())
                ])
            }
        );
    }
}
