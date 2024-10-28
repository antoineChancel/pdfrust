#[test]
fn test_helloworld() {
    let file = std::fs::read("data/helloworld.pdf").unwrap();
    pdfrust::xref::xref_table(&file);
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    trailer.extract();
}

#[test]
fn test_sample() {
    let file = std::fs::read("data/sample.pdf").unwrap();
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    trailer.extract();
}

#[test]
fn test_tracemonkey() {
    let file = std::fs::read("data/tracemonkey.pdf").unwrap();
    let xref = pdfrust::xref::xref_table(&file);
    let trailer = pdfrust::trailer(&file, &xref);
    trailer.extract();
}
