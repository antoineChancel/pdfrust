use crate::{
    algebra::Number,
    filters::flate_decode,
    object::{Lemmatizer, Object},
    tokenizer::{Token, Tokenizer},
};

use super::object;
use core::panic;
use std::{collections::HashMap, iter::Peekable};

type Offset = usize;

#[derive(Debug, PartialEq)]
pub struct XrefTable(HashMap<object::IndirectObject, Offset>);

impl Default for XrefTable {
    fn default() -> Self {
        Self::new()
    }
}

impl XrefTable {
    pub fn new() -> Self {
        XrefTable(HashMap::new())
    }

    pub fn get(&self, key: &object::IndirectObject) -> Option<&Offset> {
        self.0.get(key)
    }

    pub fn get_and_fix(&self, key: &object::IndirectObject, bytes: &[u8]) -> Option<Offset> {
        match self.get(key) {
            Some(offset) => {
                let mut pattern = format!("{} {} obj", key.0, key.1).as_bytes().to_owned();
                // xref address is correct
                if bytes[*offset..].starts_with(&pattern) {
                    Some(*offset)
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
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, PartialEq)]
pub struct XrefEntry {
    number: usize,
    generation: usize,
    in_use: bool,
}

fn xref_table_subsection_entry(tokenizer: &mut Peekable<Tokenizer>) -> Option<XrefEntry> {
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

fn xref_table_subsection(tok: &mut Peekable<Tokenizer>) -> XrefTable {
    let mut table = XrefTable(HashMap::new());

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
        match xref_table_subsection_entry(tok) {
            Some(o) => {
                if o.in_use {
                    table.0.insert((object_idx, o.generation as i32), o.number);
                }
            }
            None => panic!("Unable to read xref entry"),
        }
    }
    table
}

fn startxref(pdf_bytes: &[u8]) -> usize {
    // Idea: improve search with backward search in double ended lemmatizer
    let pattern = b"startxref";
    // Check startxref existance and unicity
    match pdf_bytes
        .windows(pattern.len())
        .filter(|&w| w == pattern)
        .count() {
            0 => panic!("PDF is corrupted, no 'startxref' bytes"),
            1 => (),
            2.. => panic!("PDF contains multiple 'startxref' bytes. Incrementally updated PDF files are currently not supported.")
        };
    let index = pdf_bytes
        .windows(pattern.len())
        .position(|w| w == pattern)
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

#[derive(Debug)]
struct XRefStream {
    size: usize,                // trailer size entry (object number used in this XRef)
    // index: Vec<(usize, usize)>, // subsection object number ranges
    // prev: Option<i32>,          // byte offset of previous xref
    w: (usize, usize, usize),         // xref stream entry sizes in bytes
    stream: Vec<u8>,
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
            // index: match value.header.get("Index") {
            //     Some(Object::Array(a)) => a.windows(2).map(|subsection| (
            //         match subsection[0] {
            //             Object::Numeric(Number::Integer(n)) => n as usize,
            //             _ => panic!()
            //         },
            //         match subsection[1] {
            //             Object::Numeric(Number::Integer(n)) => n as usize,
            //             _ => panic!()
            //         })).collect(),
            //     Some(o) => panic!("Cross reference stream dictionnary contains a Index value with wrong type, found {o:?}"),
            //     None => vec![(0, size)] // default value (cf page 108)
            // },
            // prev: match value.header.get("Prev") {
            //     Some(Object::Numeric(Number::Integer(n))) => Some(*n),
            //     Some(o) => panic!("Cross reference stream dictionnary contains a Prev value with wrong type, found {o:?}"),
            //     None => None
            // },
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
            stream: flate_decode(&value.bytes)
        }
    }
}

pub fn xref_parse(xref_stream: &[u8]) -> XrefTable {
    fn num(bytes: &[u8]) -> usize {
        let mut res: usize = 0;
        for b in bytes {
            res = res * 256 + *b as usize
        }
        res
    }

    let mut tok = Tokenizer::new(xref_stream, 0).peekable();

    match tok.peek() {
        Some(Token::String(s)) => {
            // Xref table content
            if s.as_slice() == b"xref" {
                tok.next(); // skip
                xref_table_subsection(&mut tok)
            } else {
                panic!("Startxref string missing in tokenizer, found token {s:?}")
            }
        }
        Some(Token::Numeric(_)) => {
            match Object::try_from(&mut Lemmatizer::new(xref_stream, 0, &XrefTable::new())) {
                Ok(Object::Stream(s)) => {
                    let mut xref_table = XrefTable::new();
                    let xref_stream = XRefStream::from(s);
                    let entry_size = xref_stream.w.0 + xref_stream.w.1 + xref_stream.w.2;
                    for object_idx in 0..xref_stream.size {
                        let entry = &xref_stream.stream
                            [object_idx * entry_size..object_idx * entry_size + entry_size];
                        // cross reference entries in page 109
                        let entry_type = num(&entry[..xref_stream.w.0]);
                        match entry_type {
                            1 => {
                                let object_offset = num(&entry[xref_stream.w.0..xref_stream.w.0+xref_stream.w.1]);
                                let object_generation = num(&entry[xref_stream.w.1..]);
                                xref_table.0.insert((object_idx as i32, object_generation as i32), object_offset);
                            },
                            0 => (), // not implemented yet
                            2 => (), // not implemented yet
                            _ => panic!("Cross reference stream data type can only be 0, 1 or 2")
                        }
                        
                    };
                    xref_table
                }
                Ok(o) => panic!("Xref object cannot be of type {o:?}"),
                Err(s) => panic!("{s:?}"),
            }
        }
        Some(_t) => panic!("Xref object or strign 'xref' not found"),
        None => panic!("End of stream"),
    }
}

// Parse PDF xref table and previous
pub fn xref_table(file_stream: &[u8]) -> XrefTable {
    let startxref = startxref(file_stream);
    xref_parse(&file_stream[startxref..])
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn xref_valid_entry_in_use() {
        let mut entry = Tokenizer::new(b"0000000010 00000 n", 0).peekable();
        assert_eq!(
            xref_table_subsection_entry(&mut entry).unwrap(),
            XrefEntry {
                number: 10,
                generation: 0,
                in_use: true
            }
        );
    }

    #[test]
    fn xref_valid_entry_not_in_use() {
        let mut entry = Tokenizer::new(b"0000000000 65535 f", 0).peekable();
        assert_eq!(
            xref_table_subsection_entry(&mut entry).unwrap(),
            XrefEntry {
                number: 0,
                generation: 65535,
                in_use: false
            }
        );
    }

    #[test]
    fn xref_table_valid() {
        let xref_sample = b"xref\n0 6\n0000000000 65535 f \n0000000010 00000 n \n0000000079 00000 n \n0000000173 00000 n \n0000000301 00000 n \n0000000380 00000 n";
        let table = xref_parse(xref_sample);
        assert_eq!(table.len(), 5);
        assert_eq!(table.get(&(1, 0)), Some(&10));
        assert_eq!(table.get(&(2, 0)), Some(&79));
        assert_eq!(table.get(&(5, 0)), Some(&380));
    }
}
