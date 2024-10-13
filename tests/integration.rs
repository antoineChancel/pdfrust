#[test]
fn test_hello_world() {
    let file = std::fs::read("data/helloworld.pdf").unwrap();
    pdfrust::trailer(&file);
}