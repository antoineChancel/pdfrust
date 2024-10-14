#[test]
fn test_helloworld() {
    let file = std::fs::read("data/helloworld.pdf").unwrap();
    pdfrust::trailer(&file);
    // pdfrust::xref_table(&file);
}

#[test]
fn test_sample() {
    let file = std::fs::read("data/sample.pdf").unwrap();
    // pdfrust::xref_table(&file);
    // pdfrust::trailer(&file);
}

#[test]
fn test_tracemonkey() {
    let file = std::fs::read("data/tracemonkey.pdf").unwrap();
    // pdfrust::xref_table(&file);
    // pdfrust::trailer(&file);
}
