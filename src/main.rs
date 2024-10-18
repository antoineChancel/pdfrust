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

    // Cross reference table extraction
    pdfrust::xref::xref_table(&file);

    // Trailer
    let trailer = pdfrust::trailer(&file);
    println!("{trailer:?}");

    // Information
    // match trailer.info {
    //     Some(info) => {
    //         let info_idx = XREF.get(&info).unwrap();
    //         let info = pdfrust::info(&file[*info_idx..]);
    //         println!("{info}");
    //     }
    //     None => println!("No info dictionary found"),
    // }

    // Catalog
    // let catalog_idx = XREF.get(&trailer.root).unwrap();
    // let catalog = pdfrust::catalog(&file[*catalog_idx..]);
    // println!("{catalog:?}");

    // Pages
    // let pages_idx: &usize = xref.get(&catalog.pages.unwrap()).unwrap();
    // let pages = pdfrust::pages(&file[*pages_idx..], &xref);
    // println!("{pages:?}");
}
