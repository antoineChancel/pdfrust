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
                    "--chars" => pdfrust::Extract::Chars,
                    "--font" => pdfrust::Extract::Font,
                    "--raw-content" => pdfrust::Extract::RawContent,
                    f => panic!("Invalid flag: {f}\nPdfRust currently support:\n\t--text\t\tformatted text\n\t--chars\t\ttext character font and positionning\n\t--raw-content\traw pdf content\n\t--font\t\tfont analyzer"),
                },
            },
            _ => panic!("CLI should have 2 or 3 arguments"),
        }
    }
}

fn main() {
    let config = Config::new(env::args());
    let file = std::fs::read(config.path).unwrap();
    let pdf = pdfrust::Pdf::from(file);
    let content = pdf.extract(config.flags);
    println!("{content}");
}
