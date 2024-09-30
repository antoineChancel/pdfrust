use std::{env, fs};

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

struct Metadata {
    version: PdfVersion,
}

struct IndirectObject {
    number: usize,
    generation: usize,
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

struct Token {
    chars: Vec<CharacterSet>,
}

impl Token {
    fn new() -> Token {
        Token { chars: Vec::new() }
    }

    fn add(&mut self, value: CharacterSet) {
        self.chars.push(value);
    }
}

fn pdf_version(s: &[u8]) -> PdfVersion {
    match &s[s.len()-3..] {
        b"1.7" => PdfVersion::v_1_7,
        b"1.4" => PdfVersion::v_1_4,
        _ => panic!("Pdf version not supported")
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
    if &file[file.len()-5..] != b"%%EOF" {
        panic!("PDF file is corrupted; not consistent trailing charaters");
    }

    // reading file data in 8 bits (page 48 of specs)
    for char in file.iter() {
        // character set : whitespaces, delimiters and regulars
        let char_type = match char {
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
        };
    }
}
