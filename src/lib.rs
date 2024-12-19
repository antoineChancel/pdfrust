use std::{fmt::Display, rc::Rc};

use xref::XRef;

pub mod algebra;
pub mod body;
pub mod cmap;
pub mod content;
pub mod filters;
pub mod info;
pub mod object;
pub mod tokenizer;
pub mod xref;

#[derive(Debug, Clone)]
pub enum Extract {
    Text,
    Chars,
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

pub struct Pdf {
    xref: xref::XRef,
}

impl From<Vec<u8>> for Pdf {
    fn from(value: Vec<u8>) -> Self {
        // remove leading and trailing whitespaces
        let file = value.trim_ascii();
        // check file bytes ends with %%EOF
        if &file[file.len() - 5..] != b"%%EOF" {
            panic!("PDF file is corrupted; not consistent trailing charaters");
        }
        let startxref = xref::startxref(&value);

        Pdf {
            xref: XRef::new(value.as_slice(), startxref),
        }
    }
}

impl Pdf {
    pub fn extract(&self, e: Extract) -> String {
        String::new()
    }

    pub fn read_catalog(file_stream: &[u8], curr_idx: usize, xref: Rc<xref::XRef>) -> body::Catalog {
        body::Catalog::new(file_stream, curr_idx, xref)
    }
    
    pub fn read_info(file_stream: &[u8], curr_idx: usize, xref: Rc<xref::XRef>) -> info::Info {
        info::Info::new(file_stream, curr_idx, xref)
    }
}
