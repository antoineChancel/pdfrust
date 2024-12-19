use crate::{
    algebra::Number,
    filters::flate_decode,
    object::Object,
    tokenizer::{Token, Tokenizer},
};

use super::object;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Clone)]
pub enum XRef {
    XRefTable(XRefTable),
    XRefStream(XRefStream),
}

impl XRef {
    pub fn get_and_fix(&self, key: &object::IndirectObject, bytes: &[u8]) -> Option<usize> {
        match self {
            XRef::XRefStream(stream) => stream.get(key),
            XRef::XRefTable(xref) => xref.get_and_fix(key, bytes),
        }
    }

    pub fn get(&self, key: &object::IndirectObject) -> Option<usize> {
        match self {
            XRef::XRefStream(stream) => stream.get(key),
            XRef::XRefTable(xref) => xref.get(key),
        }
    }

    pub fn get_catalog_offset(&self) -> Option<usize> {
        match self {
            XRef::XRefStream(stream) => stream.get_catalog_offset(),
            XRef::XRefTable(table) => table.get_catalog_offset(),
        }
    }

    pub fn new(bytes: &[u8], offset: usize) -> Self {
        Self::from(Tokenizer::new(bytes, offset))
    }
}

impl From<Tokenizer<'_>> for XRef {
    fn from(value: Tokenizer<'_>) -> Self {
        match value.clone().peekable().peek() {
            // Cross reference table starts with "xref" token (page 93)
            Some(Token::String(_)) => XRef::XRefTable(XRefTable::from(value)),
            // Cross reference stream object starts with "0 0 obj"
            Some(Token::Numeric(_)) => match Object::from(value) {
                Object::Stream(s) => XRef::XRefStream(XRefStream::from(s)),
                o => panic!("Xref object cannot be of type {o:?}"),
            },
            Some(_t) => panic!("Xref object or strign 'xref' not found"),
            None => panic!("End of stream"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct XRefTable {
    // Trailer - Object number
    size: Number,
    // Trailer - Byte offset from the beginning of the file to the beginning of the previous cross-reference section
    prev: Option<Box<XRef>>,
    // Trailer - Catalogue dictionnary or a reference to the root object of the page tree
    root: Option<(i32, i32)>,
    // Trailer - Encryption dictionnary
    encrypt: Option<(i32, i32)>,
    // Trailer - Information dictionary containing metadata
    info: Option<(i32, i32)>,
    // Trailer - Array of two byte-strings constituting a file identifier
    // id: Option<Array<'a>>,
    // XRef table data
    table: HashMap<object::IndirectObject, (usize, bool)>,
}

impl From<Tokenizer<'_>> for XRefTable {
    fn from(mut tokenizer: Tokenizer<'_>) -> Self {
        // Check that xref table starts with "xref" bytes
        match tokenizer.next() {
            Some(Token::String(s)) => {
                if s.as_slice() != b"xref" {
                    panic!("Startxref string missing, found string {s:?}")
                }
            }
            Some(t) => panic!(
                "Incorrect token found at the beginning of XRefTable, found {:?}",
                t
            ),
            None => panic!("End of file unexpected"),
        };

        // Read table subsections
        let table = XRefTable::read_table_subsection(&mut tokenizer);

        // Check that xref table trailer is starting with "trailer" bytes
        match tokenizer.next() {
            Some(Token::String(s)) => {
                if s.as_slice() != b"trailer" {
                    panic!("Trailer string missing, found string {s:?}")
                }
            }
            Some(t) => panic!(
                "Incorrect token found at the beginning of trailer, found {:?}",
                t
            ),
            None => panic!("End of file"),
        };

        // Read trailer dictionnary
        let trailer = match Object::from(tokenizer.clone()) {
            Object::Dictionary(dict) => dict,
            _ => panic!("Trailer should be a dictionary"),
        };

        XRefTable {
            size: match trailer.get("Size") {
                Some(Object::Numeric(n)) => n.clone(),
                _ => panic!("Size should be a numeric"),
            },
            // Byte offset from the beginning of the file to the beginning of the previous cross-reference section
            prev: match trailer.get("Prev") {
                Some(Object::Numeric(Number::Integer(offset))) => Some(Box::new(XRef::from(
                    Tokenizer::new(tokenizer.bytes, *offset as usize),
                ))),
                None => None,
                _ => panic!("Prev should be a numeric"),
            },
            // Catalogue dictionnary or a reference to the root object of the page tree
            root: match trailer.get("Root") {
                Some(Object::Ref(r, _, _)) => Some(*r),
                _ => panic!("Root should be a Catalog object"),
            },
            // Encryption dictionnary
            encrypt: match trailer.get("Encrypt") {
                Some(Object::Ref((obj, gen), _xref, _bytes)) => Some((*obj, *gen)),
                None => None,
                _ => panic!("Encrypt should be an indirect object"),
            },
            // Information dictionary containing metadata
            info: match trailer.get("Info") {
                Some(Object::Ref(r, _, _)) => Some(*r),
                None => None,
                _ => panic!("Info should be an indirect object"),
            },
            // Array of two byte-strings constituting a file identifier
            // id: Option<Array<'a>>,
            table,
        }
    }
}

impl Default for XRefTable {
    fn default() -> Self {
        XRefTable {
            size: Number::Integer(0),
            prev: None,
            root: None,
            encrypt: None,
            info: None,
            // id: None,
            table: HashMap::new(),
        }
    }
}

impl XRefTable {
    fn read_subsection_entry(tokenizer: &mut Tokenizer) -> Option<XrefEntry> {
        // either the next obj num if free or byte offset if in use
        let number = match tokenizer.next() {
            Some(Token::Numeric(Number::Integer(n))) => n as usize,
            Some(t) => panic!("Xref entry offset token should be an integer, found {t:?}"),
            None => panic!("Xref entry incomplete"),
        };

        let generation = match tokenizer.next() {
            Some(Token::Numeric(Number::Integer(n))) => n as usize,
            Some(t) => panic!("Xref entry generation token should be an integer, found {t:?}"),
            None => panic!("Xref entry incomplete"),
        };

        let in_use = match tokenizer.next() {
            Some(Token::String(s)) => s == b"n".to_vec(),
            Some(t) => panic!("Xref entry in_use token should be a regular string, found {t:?}"),
            None => panic!("Xref entry incomplete"),
        };

        Some(XrefEntry {
            number,
            generation,
            in_use,
        })
    }

    fn read_table_subsection(
        tok: &mut Tokenizer,
    ) -> HashMap<object::IndirectObject, (usize, bool)> {
        let mut table = HashMap::new();

        let start = match tok.next() {
            Some(Token::Numeric(Number::Integer(n))) => n,
            Some(t) => panic!("Table subsection header start should be an integer, found {t:?}"),
            None => panic!("Unable to read table subsection header"),
        };

        let size = match tok.next() {
            Some(Token::Numeric(Number::Integer(n))) => n,
            Some(t) => panic!("Table subsection header size should be an integer, found {t:?}"),
            None => panic!("Unable to read table subsection header"),
        };

        for object_idx in start..start + size {
            match XRefTable::read_subsection_entry(tok) {
                Some(o) => {
                    table.insert((object_idx, o.generation as i32), (o.number, o.in_use));
                }
                None => panic!("Unable to read xref entry"),
            }
        }
        table
    }

    pub fn get_catalog_offset(&self) -> Option<usize> {
        println!("{:?}", self.root.unwrap());
        self.get(&self.root.unwrap())
    }

    pub fn get(&self, key: &object::IndirectObject) -> Option<usize> {
        match self.table.get(key) {
            Some(v) => {
                if v.1 {
                    Some(v.0)
                } else {
                    panic!("XReftable object was freed")
                }
            }
            None => match &self.prev {
                // look for the key in previous xref section
                Some(xref) => xref.get(key),
                None => None,
            },
        }
    }

    pub fn get_and_fix(&self, key: &object::IndirectObject, bytes: &[u8]) -> Option<usize> {
        match self.get(key) {
            Some(offset) => {
                let mut pattern = format!("{} {} obj", key.0, key.1).as_bytes().to_owned();
                // xref address is correct
                if bytes[offset..].starts_with(&pattern) {
                    Some(offset)
                // xref table adress is broken
                } else {
                    // add a new line at the beginning of the pattern to avoid matching 11 0 obj with 1 0 obj
                    pattern.insert(0, b'\n');
                    // look for object header in byte stream
                    Some(
                        bytes
                            .windows(pattern.len())
                            .position(|w: &[u8]| w == pattern)
                            .unwrap()
                            + 1,
                    )
                }
            }
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }
}

#[derive(Debug, PartialEq)]
pub struct XrefEntry {
    number: usize,
    generation: usize,
    in_use: bool,
}

pub fn startxref(pdf_bytes: &[u8]) -> usize {
    // Idea: improve search with backward search in double ended lemmatizer
    let pattern = b"startxref";
    // Index of last occurence of startxref in file bytes
    let index = pdf_bytes
        .windows(pattern.len())
        .rposition(|w| w == pattern)
        .unwrap();
    let mut tok: Tokenizer<'_> = Tokenizer::new(pdf_bytes, index);
    match tok.next() {
        Some(Token::String(s)) => {
            if s.as_slice() != b"startxref" {
                panic!("Startxref string missing in tokenizer, found token string {s:?}")
            }
        }
        Some(t) => panic!("Startxref string missing in tokenizer, found token {t:?}"),
        None => panic!("End of stream"),
    };
    match tok.next() {
        Some(Token::Numeric(Number::Integer(i))) => i as usize,
        Some(t) => panic!("Startxref integer missing in tokenizer, found token {t:?}"),
        None => panic!("End of stream"),
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct XRefStream {
    size: usize,              // trailer size entry (object number used in this XRef)
    index: (usize, usize),    // subsection object number ranges
    prev: Option<i32>,        // byte offset of previous xref
    w: (usize, usize, usize), // xref stream entry sizes in bytes
    stream: Vec<u8>,          // uncompressed xref entries
}

impl XRefStream {
    // convert slice of entry bytes to numbers
    // high bytes first
    fn num(bytes: &[u8]) -> usize {
        let mut res: usize = 0;
        for b in bytes {
            res = res * 256 + *b as usize
        }
        res
    }

    pub fn get_catalog_offset(&self) -> Option<usize> {
        Some(0)
    }

    pub fn get(&self, key: &object::IndirectObject) -> Option<usize> {
        let object_idx = key.0 as usize;
        // check that object number is in index range
        if object_idx > self.index.1 {
            panic!("Object number {:?} is out of index", key.0)
        }
        let entry_size = self.w.0 + self.w.1 + self.w.2;
        let entry = &self.stream[object_idx * entry_size..object_idx * entry_size + entry_size];
        println!("{entry:?}");
        // cross reference entries in page 109
        let entry_type = XRefStream::num(&entry[..self.w.0]);
        let entry_mid = XRefStream::num(&entry[self.w.0..self.w.0 + self.w.1]);
        match entry_type {
            1 => Some(entry_mid),
            0 => None,                             // not implemented yet - freed objects
            2 => self.get(&(entry_mid as i32, 0)), // not implemented yet - compressed object
            _ => panic!("Cross reference stream data type can only be 0, 1 or 2"),
        }
    }
}

impl From<object::Stream<'_>> for XRefStream {
    fn from(value: object::Stream<'_>) -> Self {
        let size = match value.header.get("Size") {
            Some(Object::Numeric(Number::Integer(n))) => *n as usize,
            Some(o) => panic!(
                "Cross reference stream dictionnary contains a Size with wrong type, found {o:?}"
            ),
            None => {
                panic!("Cross reference stream dictionnary does not contains the required Size key")
            }
        };

        match value.header.get("DecodeParms") {
            Some(Object::Dictionary(_)) => {
                panic!("Data encoded with custom filters which is currently not supported")
            }
            Some(decode_parms_object) => {
                panic!("DecodeParams should be a dictionnary, found {decode_parms_object:?}")
            }
            None => (),
        };

        XRefStream {
            size,
            index: match value.header.get("Index") {
                Some(Object::Array(a)) => {
                    if a.len() != 2 {
                        panic!("Cross reference stream key 'Index' is not an array of length 2");
                    }
                    (
                        match a[0] {
                            Object::Numeric(Number::Integer(n)) => n as usize,
                            _ => panic!()
                        },
                        match a[1] {
                            Object::Numeric(Number::Integer(n)) => n as usize,
                            _ => panic!()
                        }
                    )
                }
                Some(o) => panic!("Cross reference stream dictionnary contains a Index value with wrong type, found {o:?}"),
                None => (0, size) // default value (cf page 108)
            },
            prev: match value.header.get("Prev") {
                Some(Object::Numeric(Number::Integer(n))) => Some(*n),
                Some(o) => panic!("Cross reference stream dictionnary contains a Prev value with wrong type, found {o:?}"),
                None => None
            },
            w: match value.header.get("W") {
                Some(Object::Array(a)) => {
                    (
                        match &a[0] {
                            Object::Numeric(Number::Integer(n)) => *n as usize,
                            o => panic!("Cross reference stream dictionnary Index subsection indexes should be numbers, found {o:?}")
                        },
                        match &a[1] {
                            Object::Numeric(Number::Integer(n)) => *n as usize,
                            o => panic!("Cross reference stream dictionnary Index subsection indexes should be numbers, found {o:?}")
                        },
                        match &a[2] {
                            Object::Numeric(Number::Integer(n)) => *n as usize,
                            o => panic!("Cross reference stream dictionnary Index subsection indexes should be numbers, found {o:?}")
                        },
                    )
                },
                Some(o) => panic!("Cross reference stream dictionnary key W should contain an array, found {o:?}"),
                None => panic!("Cross reference stream dictionnary key W is required")
            },
            // header: &value.header,
            stream: flate_decode(&value.bytes)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn xref_valid_entry_in_use() {
        let mut entry = Tokenizer::new(b"0000000010 00000 n", 0);
        assert_eq!(
            XRefTable::read_subsection_entry(&mut entry).unwrap(),
            XrefEntry {
                number: 10,
                generation: 0,
                in_use: true
            }
        );
    }

    #[test]
    fn xref_valid_entry_not_in_use() {
        let mut entry = Tokenizer::new(b"0000000000 65535 f", 0);
        assert_eq!(
            XRefTable::read_subsection_entry(&mut entry).unwrap(),
            XrefEntry {
                number: 0,
                generation: 65535,
                in_use: false
            }
        );
    }

    #[test]
    fn xref_table_valid() {
        let xref_sample = b"xref\n0 6\n0000000000 65535 f \n0000000010 00000 n \n0000000079 00000 n \n0000000173 00000 n \n0000000301 00000 n \n0000000380 00000 n \ntrailer\n<<\n  /Size 6\n  /Root 1 0 R\n>>";
        let table = match XRef::new(xref_sample, 0) {
            XRef::XRefTable(t) => t,
            XRef::XRefStream(_) => panic!(),
        };
        assert_eq!(table.len(), 6);
        assert_eq!(table.get(&(1, 0)), Some(10));
        assert_eq!(table.get(&(2, 0)), Some(79));
        assert_eq!(table.get(&(5, 0)), Some(380));
    }

    #[test]
    fn xref_stream_valid() {
        let xref_sample = b"22 0 obj\n<<\n /Type /XRef\n/Index [0 23]\n/Size 23\n/W [1 2 1]\n/Root 20 0 R\n/Info 21 0 R\n/ID [<8EBF2018CB18810B2C88BDD4E7324774> <8EBF2018CB18810B2C88BDD4E7324774>]\n/Length 0        \n/Filter /FlateDecode\n>>\nstream\n\nendstream\nendobj";
        match XRef::new(xref_sample, 0) {
            XRef::XRefStream(t) => t,
            XRef::XRefTable(_) => panic!(),
        };
    }
}
