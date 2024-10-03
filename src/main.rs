use core::str;
use std::{
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
    let mut iter = line.split_whitespace();
    let xref_sub_first_obj = match iter.next() {
        Some(w) => { usize::from_str_radix(w, 10).unwrap() },
        None => { return None }
    };
    let xref_sub_nb_obj = match iter.next() {
        Some(w) => { usize::from_str_radix(w, 10).unwrap() },
        None => { return None }
    };
    Some((xref_sub_first_obj, xref_sub_nb_obj))
}

fn xref_table(file: &[u8]) {
    // Address of xref / readaing file in reverse order / trying to match first decimal number
    let xref_idx = xref_address(&file);
    // Extract xref table
    for (idx, l) in str::from_utf8(&file[xref_idx..]).unwrap().lines().enumerate() {
        // End condition
        if l.contains("startxref") {
            break;
        } else {
            // Each cross-reference section begins with a line containing the keyword xref
            if idx == 0 && l != "xref" {
                panic!("Xref table first line should be xref, found {l}");
            } else {
                xref_table_subsection_header(l);
            }
        }
    }
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
    fn extract_xref_adress() {
        let end_chars = b"startxref\n\r492\n\r%%EOF";
        let result = xref_address(end_chars);
        assert_eq!(result, 492);
    }

    #[test]
    fn extract_xref_subsection_header() {
        let s = "28 4";
        let (first, size) = xref_table_subsection_header(s).unwrap();
        assert_eq!(first, 28);
        assert_eq!(size, 4)
    }
}
