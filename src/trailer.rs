use crate::{
    algebra::Number,
    body::Catalog,
    info::Info,
    object::{Array, Dictionary, IndirectObject, Object},
    xref::XrefTable,
    Extract,
};

// Trailer structure
#[derive(Debug)]
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

    pub fn extract(&self, e: Extract) -> String {
        match &self.root {
            Some(catalog) => catalog.extract(e),
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
                Some(Object::Ref((obj, gen), xref, bytes)) => xref
                    .get_and_fix(&(*obj, *gen), bytes)
                    .map(|address| Catalog::new(bytes, address, xref)),
                _ => panic!("Root should be a Catalog object"),
            },
            encrypt: match value.get("Encrypt") {
                Some(Object::Ref((obj, gen), _xref, _bytes)) => Some((*obj, *gen)),
                None => None,
                _ => panic!("Encrypt should be an indirect object"),
            },
            info: match value.get("Info") {
                Some(Object::Ref((obj, gen), xref, bytes)) => xref
                    .get_and_fix(&(*obj, *gen), bytes)
                    .map(|address| Info::new(bytes, address, xref)),
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
        let trailer = Trailer::new(bytes, 0, &xref);
        assert_eq!(trailer.size, Number::Integer(6));
        assert!(trailer.root.is_none());
        assert!(trailer.info.is_none());
        assert!(trailer.prev.is_none());
        assert!(trailer.encrypt.is_none());
    }

    #[test]
    fn read_trailer_from_one_line() {
        // Array is not read correctly -> to fix
        let bytes =
            b"<< /Size 26 /Root 13 0 R /Info 1 0 R /ID [ <4e949515aaf132498f650e7bde6cdc2f>\n<4e949515aaf132498f650e7bde6cdc2f> ] >>"
                .as_slice();
        let xref = XrefTable::new();
        let trailer = Trailer::new(bytes, 0, &xref);
        assert_eq!(trailer.size, Number::Integer(26));
        assert!(trailer.root.is_none());
        assert!(trailer.info.is_none());
        assert!(trailer.prev.is_none());
        assert!(trailer.encrypt.is_none());
        assert_eq!(
            trailer.id,
            Some(vec![
                Object::HexString(
                    [78, 148, 149, 21, 170, 241, 50, 73, 143, 101, 14, 123, 222, 108, 220, 47]
                        .to_vec()
                ),
                Object::HexString(
                    [78, 148, 149, 21, 170, 241, 50, 73, 143, 101, 14, 123, 222, 108, 220, 47]
                        .to_vec()
                ),
            ])
        );
    }
}
