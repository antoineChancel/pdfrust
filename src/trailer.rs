use crate::{
    body::Catalog,
    info::Info,
    object::{Array, Dictionary, IndirectObject, Number, Object},
    xref::XrefTable,
};

// Trailer structure
#[derive(Debug, PartialEq)]
pub struct Trailer<'a> {
    // Total number of entries in the file’s cross-reference table
    size: Number,
    // Byte offset from the beginning of the file to the beginning of the previous cross-reference section
    prev: Option<Number>,
    // Catalogue dictionnary or a reference to the root object of the page tree
    pub root: Option<Catalog>,
    // Encryption dictionnary
    encrypt: Option<IndirectObject>,
    // Information dictionary containing metadata
    pub info: Option<Info>,
    // Array of two byte-strings constituting a file identifier
    id: Option<Array<'a>>,
}

impl<'a> Trailer<'a> {
    pub fn new(bytes: &'a [u8], curr_idx: usize, xref: &'a XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }

    pub fn extract(&self) -> String {
        // Extract text
        match &self.root {
            Some(catalog) => catalog.extract(),
            None => panic!("Root object is empty"),
        }
    }
}

impl<'a> From<Dictionary<'a>> for Trailer<'a> {
    fn from(value: Dictionary<'a>) -> Self {
        Trailer {
            size: match value.get("Size") {
                Some(Object::Numeric(n)) => n.clone(),
                _ => panic!("Size should be a numeric"),
            },
            prev: match value.get("Prev") {
                Some(Object::Numeric(n)) => Some(n.clone()),
                None => None,
                _ => panic!("Prev should be a numeric"),
            },
            root: match value.get("Root") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Some(Catalog::new(&bytes, address, xref)),
                        None => None,
                    }
                }
                _ => panic!("Root should be a Catalog object"),
            },
            encrypt: match value.get("Encrypt") {
                Some(Object::Ref((obj, gen), _xref, _bytes)) => Some((*obj, *gen)),
                None => None,
                _ => panic!("Encrypt should be an indirect object"),
            },
            info: match value.get("Info") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Some(Info::new(&bytes, address, xref)),
                        None => None,
                    }
                }
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
        let bytes = b"<<\n  /Size 6\n  /Root 1 0 R\n>>".as_slice();
        let xref = XrefTable::new();
        assert_eq!(
            Trailer::new(bytes, 0, &xref),
            Trailer {
                size: Number::Integer(6),
                root: None,
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
        let bytes =
            b"<< /Size 26 /Root 13 0 R /Info 1 0 R /ID [ <4e949515aaf132498f650e7bde6cdc2f>\n<4e949515aaf132498f650e7bde6cdc2f> ] >>"
                .as_slice();
        let xref = XrefTable::new();
        assert_eq!(
            Trailer::new(bytes, 0, &xref),
            Trailer {
                size: Number::Integer(26),
                root: None,
                info: None,
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
