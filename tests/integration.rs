#[test]
fn test_helloworld() {
    let file = std::fs::read("data/helloworld.pdf").unwrap();
    pdfrust::xref::xref_table(&file);
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    assert_eq!(trailer.extract(pdfrust::Extract::Text), "Hello, world!\n");
    assert_eq!(
        trailer.extract(pdfrust::Extract::RawContent),
        "BT\n70 50 TD\n/F1 12 Tf\n(Hello, world!) Tj\nET\n"
    );
}

#[test]
fn test_sample() {
    let file = std::fs::read("data/sample.pdf").unwrap();
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    trailer.extract(pdfrust::Extract::Text);
    trailer.extract(pdfrust::Extract::RawContent);
}

#[test]
fn test_tracemonkey() {
    let file = std::fs::read("data/tracemonkey.pdf").unwrap();
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    trailer.extract(pdfrust::Extract::Text);
    trailer.extract(pdfrust::Extract::RawContent);
}

#[test]
fn test_libreoffice() {
    let file = std::fs::read("data/002-trivial-libre-office-writer.pdf").unwrap();
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    trailer.extract(pdfrust::Extract::Text);
    trailer.extract(pdfrust::Extract::RawContent);
}

#[test]
fn test_index() {
    let file = std::fs::read("data/index.pdf").unwrap();
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    trailer.extract(pdfrust::Extract::Text);
    trailer.extract(pdfrust::Extract::RawContent);
}
