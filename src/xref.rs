use crate::{algebra::Number, tokenizer::{Token, Tokenizer}};

use super::object;
use core::panic;
use std::collections::HashMap;

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
    offset: usize,
    generation: usize,
    in_use: bool,
}

// fn xref_table_subsection_header(line: &str) -> Option<(usize, usize)> {
//     // Try reading first object idx and number of object of the xref subsection
//     let mut token = line.split_whitespace();
//     let xref_sub_first_obj = match token.next() {
//         Some(w) => match w.parse::<usize>() {
//             Ok(i) => i,
//             Err(_) => return None,
//         },
//         None => return None,
//     };
//     let xref_sub_nb_obj = match token.next() {
//         Some(w) => match w.parse::<usize>() {
//             Ok(i) => i,
//             Err(_) => return None,
//         },
//         None => return None,
//     };
//     Some((xref_sub_first_obj, xref_sub_nb_obj))
// }

fn xref_table_subsection_entry(tokenizer: &mut Tokenizer) -> Option<XrefEntry> {

    let offset = match tokenizer.next() {
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
        offset,
        generation,
        in_use,
    })
}

fn xref_table_subsection(tok: &mut Tokenizer) -> XrefTable {
    let mut table = XrefTable(HashMap::new());

    let start = match tok.next() {
        Some(Token::Numeric(Number::Integer(n))) => n,
        Some(t) => panic!("Table subsection header start should be an integer, found {t:?}"),
        None => panic!("Unable to read table subsection header")
    };

    let size = match tok.next() {
        Some(Token::Numeric(Number::Integer(n))) => n,
        Some(t) => panic!("Table subsection header size should be an integer, found {t:?}"),
        None => panic!("Unable to read table subsection header")
    };

    for object_idx in start..start + size {
        match xref_table_subsection_entry(tok) {
            Some(o) => {
                table
                    .0
                    .insert((object_idx, o.generation as i32), o.offset);
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
    let mut tok = Tokenizer::new(pdf_bytes, index);
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

pub fn xref_parse(xref_stream: &[u8]) -> XrefTable {

    let mut tok = Tokenizer::new(xref_stream, 0);

    match tok.next() {
        Some(Token::String(s)) => {
            if s.as_slice() == b"xref" {
                xref_table_subsection(&mut tok)
            } else {
                panic!("Startxref string missing in tokenizer, found token {s:?}")
            }
        }
        Some(t) => panic!("Startxref string missing in tokenizer, found token {t:?}"),
        None => panic!("End of stream"),
    }
}

// Parse PDF xref table and previous
pub fn xref_table(file_stream: &[u8]) -> XrefTable {
    // Read the last startxref in the file
    let startxref = startxref(file_stream);
    xref_parse(&file_stream[startxref..])
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn xref_valid_entry_in_use() {
        let mut entry = Tokenizer::new(b"0000000010 00000 n", 0);
        assert_eq!(
            xref_table_subsection_entry(&mut entry).unwrap(),
            XrefEntry {
                offset: 10,
                generation: 0,
                in_use: true
            }
        );
    }

    #[test]
    fn xref_valid_entry_not_in_use() {
        let mut entry = Tokenizer::new(b"0000000000 65535 f", 0);
        assert_eq!(
            xref_table_subsection_entry(&mut entry).unwrap(),
            XrefEntry {
                offset: 0,
                generation: 65535,
                in_use: false
            }
        );
    }

    #[test]
    fn xref_table_valid() {
        let xref_sample = b"xref\n0 6\n0000000000 65535 f \n0000000010 00000 n \n0000000079 00000 n \n0000000173 00000 n \n0000000301 00000 n \n0000000380 00000 n";
        let table = xref_parse(xref_sample);
        assert_eq!(table.len(), 6);
        assert_eq!(table.get(&(1, 0)), Some(&10));
        assert_eq!(table.get(&(2, 0)), Some(&79));
        assert_eq!(table.get(&(5, 0)), Some(&380));
    }
}
