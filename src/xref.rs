use super::object;
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

fn xref_table_subsection_header(line: &str) -> Option<(usize, usize)> {
    // Try reading first object idx and number of object of the xref subsection
    let mut token = line.split_whitespace();
    let xref_sub_first_obj = match token.next() {
        Some(w) => match w.parse::<usize>() {
            Ok(i) => i,
            Err(_) => return None,
        },
        None => return None,
    };
    let xref_sub_nb_obj = match token.next() {
        Some(w) => match w.parse::<usize>() {
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
        Some(w) => match w.parse::<usize>() {
            Ok(i) => i,
            Err(_) => return None,
        },
        None => return None,
    };
    let generation = match token.next() {
        Some(w) => match w.parse::<usize>() {
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

fn xref_table_subsection(line: &mut std::str::Lines) -> XrefTable {
    let mut table = XrefTable(HashMap::new());

    let (start, size) = xref_table_subsection_header(line.next().unwrap()).unwrap();

    for object_idx in start..start + size {
        match xref_table_subsection_entry(line.next().unwrap()) {
            Some(o) => {
                table
                    .0
                    .insert((object_idx as i32, o.generation as i32), o.offset);
            }
            None => panic!("Unable to read xref entry"),
        }
    }
    table
}

fn xref_slice(stream: &[u8]) -> &str {
    // TODO - improve this by reading the startxref on last line
    let startxref = match stream.windows(4).position(|w| w == b"xref") {
        Some(i) => i,
        None => panic!("Missing xref token in the entire PDF"),
    };
    match std::str::from_utf8(&stream[startxref..]) {
        Ok(s) => s,
        Err(_) => panic!(
            "Unable to read xref slice, {:?}",
            std::str::from_utf8(&stream[startxref..startxref + 800])
        ),
    }
}

// Parse PDF xref table
pub fn xref_table(file_stream: &[u8]) -> XrefTable {
    // Read the cross reference table by lines
    let mut line = xref_slice(file_stream).lines();

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
        let table = xref_table(xref_sample);
        assert_eq!(table.len(), 6);
        assert_eq!(table.get(&(1, 0)), Some(&10));
        assert_eq!(table.get(&(2, 0)), Some(&79));
        assert_eq!(table.get(&(5, 0)), Some(&380));
    }
}
