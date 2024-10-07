use core::{panic, str};
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum PdfVersion {
    V1_3,
    V1_4,
    V1_7,
}

#[derive(Debug)]
enum WhiteSpace {
    Null,
    Tab,
    LineFeed,
    FormFeed,
    CarriageReturn,
    Space,
}

impl WhiteSpace {
    fn new(char: u8) -> WhiteSpace {
        match char {
            0 => WhiteSpace::Null,
            9 => WhiteSpace::Tab,
            10 => WhiteSpace::LineFeed,
            12 => WhiteSpace::FormFeed,
            13 => WhiteSpace::CarriageReturn,
            32 => WhiteSpace::Space,
            _ => panic!("Unable to interprete character set whitespace"),
        }
    }

    fn is_eol(&self) -> bool {
        match self {
            Self::LineFeed | Self::CarriageReturn => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum Delimiter {
    String,
    Array,
    Name,
    Comment,
}

impl Delimiter {
    fn new(char: u8) -> Delimiter {
        match char {
            b'(' | b')' => Delimiter::String,
            b'<' | b'>' | b'[' | b']' | b'{' | b'}' => Delimiter::Array,
            b'/' => Delimiter::Name,
            b'%' => Delimiter::Comment,
            _ => panic!("Unable to interprete character set delimiter"),
        }
    }
}

#[derive(Debug)]
enum CharacterSet {
    Regular { char: u8 },
    Delimiter { char: u8, value: Delimiter },
    WhiteSpace { char: u8, value: WhiteSpace },
}

pub fn pdf_version(s: &[u8]) -> PdfVersion {
    match &s[s.len() - 3..] {
        b"1.7" => PdfVersion::V1_7,
        b"1.4" => PdfVersion::V1_4,
        b"1.3" => PdfVersion::V1_3,
        _ => panic!("Pdf version not supported"),
    }
}

impl CharacterSet {
    fn new(char: &u8) -> CharacterSet {
        match char {
            0 | 9 | 10 | 12 | 13 | 32 => CharacterSet::WhiteSpace {
                char: *char,
                value: WhiteSpace::new(*char),
            },
            b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%' => {
                CharacterSet::Delimiter {
                    char: *char,
                    value: Delimiter::new(*char),
                }
            }
            _ => CharacterSet::Regular { char: *char },
        }
    }
}

fn startxref(bytes: &[u8]) -> usize {
    let mut res: usize = 0;
    let mut exp = 0;

    // read file bytes in reverse order
    for i in bytes[..bytes.len() - 5].iter().rev() {
        let is_digit = match CharacterSet::new(i) {
            CharacterSet::Delimiter { char, .. } => panic!(
                "Bytes before %%EOF should not be a delimiter: {}",
                char as char
            ),
            CharacterSet::WhiteSpace { char: _, value } => {
                if value.is_eol() {
                    continue;
                } else {
                    panic!("Bytes before %%EOF should not be delimiters")
                }
            }
            CharacterSet::Regular { char } => char.is_ascii_digit(),
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

struct IndirectRef {
    obj_num: usize,
    obj_gen: usize,
}

// impl IndirectRef {

//     // Read bytes into "1 0 R" IndirectRef
//     fn new(bytes: &[u8]) -> IndirectRef {

//     }
// }

struct Trailer {
    size: usize,
    prev: usize,
    root: Option<IndirectRef>,    // Catalogue dictionnary
    encrypt: Option<IndirectRef>, // Encryption dictionnary
    info: Option<IndirectRef>,    // Information dictionary
    id: Option<Vec<String>>,      // An array of two byte-strings constituting a file identifier
}

struct Name {
    value: String,
}

impl From<&[u8]> for Name {
    fn from(value: &[u8]) -> Self {
        let mut c = value.iter();
        // Name object starts with regular character /
        match CharacterSet::new(c.next().unwrap()) {
            CharacterSet::Regular { char } => {
                if char != b'/' {
                    panic!("Pdf name object should start with a /");
                }
            }
            _ => panic!("Pdf name object should start with a regular character"),
        }
        let mut name = String::new();
        loop {
            match CharacterSet::new(c.next().unwrap()) {
                CharacterSet::Regular { char } => name.push(char::from(char)),
                _ => break,
            }
        }
        Name { value: name }
    }
}

// enum DictObject<'a> {
//     Name(&'a Name),
//     Boolean(&'a bool),
//     Numeric(&'a usize),
//     String(&'a String),
//     Stream(&'a String),
//     Dict(HashMap<Name, Object<'a>>),
// }

// fn read_pdf_dictionnary (file_stream: &[u8]) -> HashMap<Name, DictObject>{
//     assert_eq!(l.next());
// }

// Parse PDF trailer
// Implementation note 13 :  Acrobat viewers require only that the header
// appear somewhere within the first 1024 bytes of the file.
pub fn trailer(file_stream: &[u8]) -> () {
    // locate trailer address
    let starttrailer = match file_stream.windows(7).position(|w| w == b"trailer") {
        Some(i) => i,
        None => panic!("Missing trailer token in the entire PDF"),
    };
    // read trailer data
    // let l = file_stream[starttrailer..].lines();
    // l.next(); // trailer
    // l.next(); // <<
    // while let Some(entry) {

    // }
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
