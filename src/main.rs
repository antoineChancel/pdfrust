use std::env;

use pdfrust::Extract;

struct Config {
    path: String,
    flags: Extract,
}

impl Config {
    fn new(args: env::Args) -> Config {
        let args: Vec<String> = args.collect();
        match args.len() {
            2 => Config {
                path: args[1].clone(),
                flags: pdfrust::Extract::Text,
            },
            3 => Config {
                path: args[2].clone(),
                flags: match args[1].as_str() {
                    "--text" => pdfrust::Extract::Text,
                    "--font" => pdfrust::Extract::Font,
                    "--raw-content" => pdfrust::Extract::RawContent,
                    _ => panic!("Invalid flag, available flags: --text, --raw-content"),
                },
            },
            _ => panic!("CLI should have 2 or 3 arguments"),
        }
    }
}

fn main() {
    let config = Config::new(env::args());
    let file = std::fs::read(config.path).unwrap();

    // Pdf header with specifications version
    let version = pdfrust::pdf_version(&file[..8]);
    println!("Pdf version: {version}");

    // Check that pdf file ends with %%EOF
    let file = file.trim_ascii();
    if &file[file.len() - 5..] != b"%%EOF" {
        panic!("PDF file is corrupted; not consistent trailing charaters");
    }

    // Cross reference table
    let xref = pdfrust::xref::xref_table(file);

    // Trailer
    let trailer = pdfrust::trailer(file, &xref);
    let text = trailer.extract(config.flags);
    println!("{text}");
}
