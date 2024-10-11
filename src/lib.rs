use core::{panic, str};
use std::collections::BTreeMap;

pub mod object;

#[derive(Debug)]
pub enum PdfVersion {
    V1_3,
    V1_4,
    V1_7,
}

pub fn pdf_version(s: &[u8]) -> PdfVersion {
    match &s[s.len() - 3..] {
        b"1.7" => PdfVersion::V1_7,
        b"1.4" => PdfVersion::V1_4,
        b"1.3" => PdfVersion::V1_3,
        _ => panic!("Pdf version not supported"),
    }
}

fn startxref(bytes: &[u8]) -> usize {
    let mut res: usize = 0;
    let mut exp = 0;

    // read file bytes in reverse order
    for i in bytes[..bytes.len() - 5].iter().rev() {
        let is_digit = match object::CharacterSet::from(i) {
            object::CharacterSet::Delimiter { char, .. } => panic!(
                "Bytes before %%EOF should not be a delimiter: {}",
                char as char
            ),
            object::CharacterSet::WhiteSpace { char: _, value } => {
                if value.is_eol() {
                    continue;
                } else {
                    panic!("Bytes before %%EOF should not be delimiters")
                }
            }
            object::CharacterSet::Regular { char } => char.is_ascii_digit(),
        };

        if is_digit {
            let digit = char::from(*i).to_digit(10).unwrap() as usize;
            res += digit * 10_usize.pow(exp);
            exp += 1;
        }

        // termination condition
        if !is_digit && res > 0 {
            break;
        }
    }
    res
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

#[derive(Debug, PartialEq)]
pub struct PdfObject {
    offset: usize,
    generation: usize,
    in_use: bool,
}

fn xref_table_subsection_entry(line: &str) -> Option<PdfObject> {
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
    Some(PdfObject {
        offset,
        generation,
        in_use,
    })
}

fn xref_table_subsection(line: &mut std::str::Lines, table: &mut BTreeMap<usize, PdfObject>) {
    let (start, size) = xref_table_subsection_header(line.next().unwrap()).unwrap();

    for object_idx in start..start + size {
        match xref_table_subsection_entry(line.next().unwrap()) {
            Some(o) => {
                table.insert(object_idx, o);
            }
            None => panic!("Unable to read xref entry"),
        }
    }
}

fn xref_slice(stream: &[u8]) -> &str {
    // Read address of xref after startxref token
    let startxref = startxref(&stream);
    println!("Pdf xref offset read is {startxref}");

    // Extract xref table with iteration on lines
    match str::from_utf8(&stream[startxref..]) {
        Ok(e) => e,
        Err(_) => {
            println!("Unable to read xref table from startxref position, PDF might be corrupted");
            println!("Looking for xref in pdf...");
            // Look for a byte chain with xref encoded
            let startxref = match stream.windows(4).position(|w| w == b"xref") {
                Some(i) => i,
                None => panic!("Missing xref token in the entire PDF"),
            };
            str::from_utf8(&stream[startxref..]).unwrap()
        }
    }
}

fn xref_table_read(mut line: core::str::Lines) -> BTreeMap<usize, PdfObject> {
    // First line should be xref
    match line.next() {
        Some("xref") => (),
        Some(s) => panic!("Xref first line contains a wrong token: {s}"),
        None => panic!("Xref table is empty"),
    };

    // Init xref table
    let mut table = BTreeMap::new();
    xref_table_subsection(&mut line, &mut table);
    table
}

// Parse PDF xref table
pub fn xref_table(file_stream: &[u8]) -> BTreeMap<usize, PdfObject> {
    xref_table_read(xref_slice(&file_stream).lines())
}

// Parse PDF trailer
// Implementation note 13 :  Acrobat viewers require only that the header
// appear somewhere within the first 1024 bytes of the file.
pub fn trailer(file_stream: &[u8]) -> () {
    // locate trailer address
    let starttrailer = match file_stream.windows(7).position(|w| w == b"trailer") {
        Some(i) => i,
        None => panic!("Missing trailer token in the entire PDF"),
    };
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn xref_adress() {
        let end_chars = b"startxref\n\r492\n\r%%EOF";
        let result = startxref(end_chars);
        assert_eq!(result, 492);
    }

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
            PdfObject {
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
            PdfObject {
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
        let xref_sample = "xref
0 6
0000000000 65535 f 
0000000010 00000 n 
0000000079 00000 n 
0000000173 00000 n 
0000000301 00000 n 
0000000380 00000 n";
        let table = xref_table_read(xref_sample.lines());
        assert_eq!(table.len(), 6);
    }
}
