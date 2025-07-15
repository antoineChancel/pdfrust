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
    Xref,
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

pub struct Pdf<'a> {
    // File bytes
    file: &'a [u8],
    // Cross reference table
    xref: xref::XRef<'a>,
}

impl<'a> From<&'a Vec<u8>> for Pdf<'a> {
    fn from(value: &'a Vec<u8>) -> Self {
        // remove leading and trailing whitespaces
        let file = value.trim_ascii();
        // check file bytes ends with %%EOF
        if &file[file.len() - 5..] != b"%%EOF" {
            panic!("PDF file is corrupted; not consistent trailing charaters");
        }
        // bytes offset of last xref table
        let startxref = xref::startxref(file);
        // read xref tables
        let xref = XRef::new(file, startxref);
        Pdf { file, xref }
    }
}

impl<'a> Pdf<'a> {
    pub fn extract(&mut self, e: Extract) -> String {
        match e {
            Extract::Xref => {
                println!("{}", self.xref);
                String::new()
            }
            _ => {
                let xref = Rc::new(self.xref.clone());
                let catalog_offset = xref.get_catalog_offset().unwrap();
                let catalog = Pdf::read_catalog(&self.file, catalog_offset);
                catalog.extract(e, &mut self.xref)
            }
        }
    }

    pub fn read_catalog(file_stream: &[u8], curr_idx: usize) -> body::Catalog {
        body::Catalog::new(file_stream, curr_idx)
    }

    pub fn read_info(file_stream: &[u8], curr_idx: usize) -> info::Info {
        info::Info::new(file_stream, curr_idx)
    }
}
