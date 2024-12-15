#[test]
fn test_helloworld() {
    let file = std::fs::read("data/helloworld.pdf").unwrap();
    let pdf = pdfrust::Pdf::from(file);
    assert_eq!(pdf.extract(pdfrust::Extract::Text), "Hello, world!");
    assert_eq!(
        pdf.extract(pdfrust::Extract::RawContent),
        "BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET\n"
    );
}

#[test]
fn test_sample() {
    let file = std::fs::read("data/sample.pdf").unwrap();
    let pdf = pdfrust::Pdf::from(file);
    pdf.extract(pdfrust::Extract::Text);
    pdf.extract(pdfrust::Extract::RawContent);
}

#[test]
fn test_tracemonkey() {
    let file = std::fs::read("data/tracemonkey.pdf").unwrap();
    let pdf = pdfrust::Pdf::from(file);
    pdf.extract(pdfrust::Extract::Text);
    pdf.extract(pdfrust::Extract::RawContent);
}

#[test]
fn test_libreoffice() {
    let file = std::fs::read("data/002-trivial-libre-office-writer.pdf").unwrap();
    let pdf = pdfrust::Pdf::from(file);
    pdf.extract(pdfrust::Extract::Text);
    pdf.extract(pdfrust::Extract::RawContent);
}

#[test]
fn test_index() {
    let file = std::fs::read("data/index.pdf").unwrap();
    let pdf = pdfrust::Pdf::from(file);
    pdf.extract(pdfrust::Extract::Text);
    pdf.extract(pdfrust::Extract::RawContent);
}

// #[test]
// fn test_latex() {
//     let file = std::fs::read("data/pdflatex-4-pages.pdf").unwrap();
//     let pdf = pdfrust::Pdf::from(file);
//     pdf.extract(pdfrust::Extract::Text);
//     pdf.extract(pdfrust::Extract::RawContent);
// }
