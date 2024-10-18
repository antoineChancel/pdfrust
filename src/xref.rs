use super::{body, info, object};
use std::cell::RefCell;
use std::collections::HashMap;
// Safe as single threaded
thread_local!(pub static XREF: RefCell<XrefTable> = RefCell::new(XrefTable::new()));

type Offset = usize;

#[derive(Debug, PartialEq)]
pub enum BodyObject {
    Catalog(body::Catalog),
    Info(info::Info),
}

#[derive(PartialEq, Debug)]
pub enum XrefValue {
    Offset(Offset),
    Object(Option<BodyObject>),
}

pub type XrefTable = HashMap<object::IndirectObject, XrefValue>;

#[derive(Debug, PartialEq)]
pub struct XrefEntry {
    offset: usize,
    generation: usize,
    in_use: bool,
}

fn xref_table_subsection_header(line: &str) -> Option<(usize, usize)> {
    // Try reading first object idx and number of object of the xref subsection
    let mut token = line.split_whitespace();
    let xref_sub_first_obj = match token.next() {
        Some(w) => match usize::from_str_radix(w, 10) {
            Ok(i) => i,
            Err(_) => return None,
        },
        None => return None,
    };
    let xref_sub_nb_obj = match token.next() {
        Some(w) => match usize::from_str_radix(w, 10) {
            Ok(i) => i,
            Err(_) => return None,
        },
        None => return None,
    };
    Some((xref_sub_first_obj, xref_sub_nb_obj))
}

fn xref_table_subsection_entry(line: &str) -> Option<XrefEntry> {
    // Try reading an xref entry
    let mut token = line.split_whitespace();
    let offset = match token.next() {
        Some(w) => match usize::from_str_radix(w, 10) {
            Ok(i) => i,
            Err(_) => return None,
        },
        None => return None,
    };
    let generation = match token.next() {
        Some(w) => match usize::from_str_radix(w, 10) {
            Ok(i) => i,
            Err(_) => return None,
        },
        None => return None,
    };
    let in_use = match token.next() {
        Some(w) => w == "n",
        None => return None,
    };
    Some(XrefEntry {
        offset,
        generation,
        in_use,
    })
}

fn xref_table_subsection(line: &mut std::str::Lines) {
    let (start, size) = xref_table_subsection_header(line.next().unwrap()).unwrap();

    for object_idx in start..start + size {
        match xref_table_subsection_entry(line.next().unwrap()) {
            Some(o) => {
                XREF.with(|xref| {
                    xref.borrow_mut().insert(
                        (object_idx as u32, o.generation as u32),
                        XrefValue::Offset(o.offset),
                    )
                });
            }
            None => panic!("Unable to read xref entry"),
        }
    }
}

fn xref_slice<'a>(stream: &'a [u8]) -> &'a str {
    // TODO - improve this by reading the startxref on last line
    let startxref = match stream.windows(4).position(|w| w == b"xref") {
        Some(i) => i,
        None => panic!("Missing xref token in the entire PDF"),
    };
    std::str::from_utf8(&stream[startxref..]).unwrap()
}

// Parse PDF xref table
pub fn xref_table(file_stream: &[u8]) {
    // Read the cross reference table by lines
    let mut line = xref_slice(&file_stream).lines();

    // First line should be xref
    match line.next() {
        Some("xref") => (),
        Some(s) => panic!("Xref first line contains a wrong token: {s}"),
        None => panic!("Xref table is empty"),
    };

    // Read xref table
    xref_table_subsection(&mut line)
}

#[cfg(test)]
mod tests {

    use std::borrow::BorrowMut;

    use super::*;

    #[test]
    fn xref_subsection_header() {
        let s = "28 4";
        let (first, size) = xref_table_subsection_header(s).unwrap();
        assert_eq!(first, 28);
        assert_eq!(size, 4)
    }

    #[test]
    fn xref_subsection_header_invalid() {
        let s = "blabla";
        assert_eq!(xref_table_subsection_header(s), None);
    }

    #[test]
    fn xref_valid_entry_in_use() {
        let entry = "0000000010 00000 n";
        assert_eq!(
            xref_table_subsection_entry(entry).unwrap(),
            XrefEntry {
                offset: 10,
                generation: 0,
                in_use: true
            }
        );
    }

    #[test]
    fn xref_valid_entry_not_in_use() {
        let entry = "0000000000 65535 f";
        assert_eq!(
            xref_table_subsection_entry(entry).unwrap(),
            XrefEntry {
                offset: 0,
                generation: 65535,
                in_use: false
            }
        );
    }

    #[test]
    fn xref_invalid_entry() {
        let entry = "<<";
        assert_eq!(xref_table_subsection_entry(entry), None);
    }

    #[test]
    fn xref_table_valid() {
        let xref_sample = b"xref\n0 6\n0000000000 65535 f \n0000000010 00000 n \n0000000079 00000 n \n0000000173 00000 n \n0000000301 00000 n \n0000000380 00000 n";
        xref_table(xref_sample);
        XREF.with(|xref| {
            let xref = xref.borrow_mut();
            assert_eq!(xref.len(), 6);
            assert_eq!(xref.get(&(1, 0)), Some(&XrefValue::Offset(10)));
            assert_eq!(xref.get(&(2, 0)), Some(&XrefValue::Offset(79)));
            assert_eq!(xref.get(&(5, 0)), Some(&XrefValue::Offset(380)));

        });
    }
}
