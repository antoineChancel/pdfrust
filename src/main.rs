use std::env;

struct Config {
    path: String,
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
    let xref = pdfrust::xref::xref_table(&file);

    // Trailer
    let trailer = pdfrust::trailer(&file, &xref);
    let text = trailer.extract();
    println!("{text}");
}
