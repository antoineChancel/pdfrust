use std::fmt::Display;

use xref::XrefTable;

pub mod body;
pub mod filters;
pub mod info;
pub mod object;
pub mod text;
pub mod tokenizer;
pub mod trailer;
pub mod xref;

#[derive(Debug, Clone)]
pub enum Extract {
    Text,
    Font,
    RawContent,
}

#[derive(Debug)]
pub enum PdfVersion {
    V1_3,
    V1_4,
    V1_5,
    V1_6,
    V1_7,
}

impl Display for PdfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfVersion::V1_3 => write!(f, "1.3"),
            PdfVersion::V1_4 => write!(f, "1.4"),
            PdfVersion::V1_5 => write!(f, "1.5"),
            PdfVersion::V1_6 => write!(f, "1.6"),
            PdfVersion::V1_7 => write!(f, "1.7"),
        }
    }
}

pub fn pdf_version(s: &[u8]) -> PdfVersion {
    match &s[s.len() - 3..] {
        b"1.7" => PdfVersion::V1_7,
        b"1.6" => PdfVersion::V1_6,
        b"1.5" => PdfVersion::V1_5,
        b"1.4" => PdfVersion::V1_4,
        b"1.3" => PdfVersion::V1_3,
        _ => panic!("Pdf version not supported"),
    }
}

// Parse PDF trailer
// Implementation note 13 :  Acrobat viewers require only that the header
// appear somewhere within the first 1024 bytes of the file.
pub fn trailer<'a>(file_stream: &'a [u8], xref: &'a XrefTable) -> trailer::Trailer<'a> {
    // locate trailer address
    let starttrailer = match file_stream.windows(7).position(|w| w == b"trailer") {
        Some(i) => i,
        None => panic!("Missing trailer token in the entire PDF"),
    };
    // slice bytes just after trailer token
    trailer::Trailer::new(file_stream, starttrailer + 8, xref)
}

pub fn catalog(file_stream: &[u8], curr_idx: usize, xref: &XrefTable) -> body::Catalog {
    body::Catalog::new(file_stream, curr_idx, xref)
}

pub fn info(file_stream: &[u8], curr_idx: usize, xref: &XrefTable) -> info::Info {
    info::Info::new(file_stream, curr_idx, xref)
}
