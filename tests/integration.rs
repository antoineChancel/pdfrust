#[test]
fn test_helloworld() {
    let file = std::fs::read("data/helloworld.pdf").unwrap();
    // pdfrust::trailer(&file);
    pdfrust::xref::xref_table(&file);
    // let catalog_idx = xref_table.get(&trailer.root).unwrap();
    // let catalog = pdfrust::catalog(&file[*catalog_idx..]);
}

#[test]
fn test_sample() {
    let file = std::fs::read("data/sample.pdf").unwrap();
    // pdfrust::trailer(&file);
    pdfrust::xref::xref_table(&file);
    // let catalog_idx = xref_table.get(&trailer.root).unwrap();
    // let catalog = pdfrust::catalog(&file[*catalog_idx..]);
}

#[test]
fn test_tracemonkey() {
    let file = std::fs::read("data/tracemonkey.pdf").unwrap();
    // pdfrust::trailer(&file);
    pdfrust::xref::xref_table(&file);
    // let catalog_idx = xref_table.get(&trailer.root).unwrap();
    // let catalog = pdfrust::catalog(&file[*catalog_idx..]);
}
