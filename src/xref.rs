use super::object;
use std::collections::HashMap;

type Offset = usize;
pub type XrefTable = HashMap<object::IndirectObject, Offset>;

#[derive(Debug, PartialEq)]
pub struct XrefVal {
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

fn xref_table_subsection_entry(line: &str) -> Option<XrefVal> {
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
    Some(XrefVal {
        offset,
        generation,
        in_use,
    })
}

fn xref_table_subsection(line: &mut std::str::Lines, table: &mut XrefTable) {
    let (start, size) = xref_table_subsection_header(line.next().unwrap()).unwrap();

    for object_idx in start..start + size {
        match xref_table_subsection_entry(line.next().unwrap()) {
            Some(o) => {
                table.insert((object_idx as u32, o.generation as u32), o.offset);
            }
            None => panic!("Unable to read xref entry"),
        }
    }
}

fn xref_slice<'a>(stream: &'a [u8]) -> &'a str {
    // improve this by reading the startxref on last line
    let startxref = match stream.windows(4).position(|w| w == b"xref") {
        Some(i) => i,
        None => panic!("Missing xref token in the entire PDF"),
    };
    std::str::from_utf8(&stream[startxref..]).unwrap()
}

// Parse PDF xref table
pub fn xref_table(file_stream: &[u8]) -> XrefTable {
    // Read the cross reference table by lines
    let mut line = xref_slice(&file_stream).lines();

    // First line should be xref
    match line.next() {
        Some("xref") => (),
        Some(s) => panic!("Xref first line contains a wrong token: {s}"),
        None => panic!("Xref table is empty"),
    };

    // Init xref table
    let mut table = HashMap::new();
    xref_table_subsection(&mut line, &mut table);
    table
}

#[cfg(test)]
mod tests {

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
            XrefVal {
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
            XrefVal {
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
        let xref_sample = b"xref
0 6
0000000000 65535 f 
0000000010 00000 n 
0000000079 00000 n 
0000000173 00000 n 
0000000301 00000 n 
0000000380 00000 n";
        let table = xref_table(xref_sample);
        assert_eq!(table.len(), 6);
    }
}
