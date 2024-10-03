use core::str;
use std::{
    collections::HashMap,
    env,
    fs::{self, read},
    io::BufRead,
};

struct Config {
    path: String,
}

#[derive(Debug)]
enum PdfVersion {
    v_1_4,
    v_1_7,
}

impl Config {
    fn new(args: env::Args) -> Config {
        let args: Vec<String> = args.collect();
        if args.len() != 2 {
            panic!("CLI should have 1 arguments")
        }
        Config {
            path: args[1].clone(),
        }
    }
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

fn pdf_version(s: &[u8]) -> PdfVersion {
    match &s[s.len() - 3..] {
        b"1.7" => PdfVersion::v_1_7,
        b"1.4" => PdfVersion::v_1_4,
        _ => panic!("Pdf version not supported"),
    }
}

fn charset(char: &u8) -> CharacterSet {
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

fn xref_address(bytes: &[u8]) -> usize {
    let mut res: usize = 0;
    let mut exp = 0;

    // read file bytes in reverse order
    for i in bytes[..bytes.len() - 5].iter().rev() {
        let is_digit = match charset(i) {
            CharacterSet::Delimiter { char, .. } => panic!(
                "Bytes before %%EOF should not be a delimiter: {}",
                char as char
            ),
            CharacterSet::WhiteSpace { char, value } => {
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
struct PdfObject {
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

fn xref_table_subsection(line: &mut std::str::Lines, table: &mut HashMap<usize, PdfObject>) {
    let (start, size) = xref_table_subsection_header(line.next().unwrap()).unwrap();

    while let Some(entry) = xref_table_subsection_entry(line.next().unwrap()) {
        println!("{entry:?}");
    }
}

fn xref_table(file: &[u8]) -> HashMap<usize, PdfObject> {
    // Address of xref in file bytes
    let xref_idx = xref_address(&file);

    // Extract xref table with iteration on lines
    let mut line = str::from_utf8(&file[xref_idx..]).unwrap().lines();

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

fn main() {
    let config = Config::new(env::args());
    let file = fs::read(config.path).unwrap();

    // Remove potential whitespaces at begin or end
    let file = file.trim_ascii();

    // Pdf file version
    let version = pdf_version(&file[..8]);
    println!("Pdf version {version:?}");

    // Pdf file has %%EOF comment
    if &file[file.len() - 5..] != b"%%EOF" {
        panic!("PDF file is corrupted; not consistent trailing charaters");
    }

    // Extract xref table
    xref_table(&file);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xref_adress() {
        let end_chars = b"startxref\n\r492\n\r%%EOF";
        let result = xref_address(end_chars);
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
        assert_eq!(xref_table_subsection_entry(entry).unwrap(), PdfObject{offset: 10, generation: 0, in_use: true});
    }

    #[test]
    fn xref_valid_entry_not_in_use() {
        let entry = "0000000000 65535 f";
        assert_eq!(
            xref_table_subsection_entry(entry).unwrap(),
            PdfObject{offset: 0, generation: 65535, in_use: false}
        );
    }

    #[test]
    fn xref_invalid_entry() {
        let entry = "<<";
        assert_eq!(xref_table_subsection_entry(entry), None);
    }
}
