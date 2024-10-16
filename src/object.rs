use core::panic;
use std::slice::Iter;

use tokenizer::{CharacterSet, Delimiter, PdfBytes, Token};

use crate::XrefTable;

mod tokenizer;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Name(String);

impl TryFrom<&mut Iter<'_, u8>> for Name {
    type Error = &'static str;

    fn try_from(value: &mut Iter<'_, u8>) -> Result<Self, Self::Error> {
        // Name object starts with regular character /'
        loop {
            match CharacterSet::from(value.next().unwrap()) {
                // Absorb eventual whitespaces before name
                CharacterSet::WhiteSpace(_) => (),
                CharacterSet::Delimiter(Delimiter::Name) => break,
                _ => return Err("Pdf name object should start with a name delimiter"),
            }
        }
        let mut name = String::new();
        loop {
            let curr = match value.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::from(curr) {
                CharacterSet::Regular(c) => name.push(char::from(c)),
                _ => break,
            }
        }
        Ok(Name(name))
    }
}

impl From<&[u8]> for Name {
    fn from(value: &[u8]) -> Self {
        let mut c = value.iter();
        // Name object starts with regular character /'
        match CharacterSet::from(c.next().unwrap()) {
            CharacterSet::Delimiter(Delimiter::Name) => (),
            _ => panic!("Pdf name object should start with a name delimiter"),
        }
        let mut name = String::new();
        loop {
            let curr = match c.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::from(curr) {
                CharacterSet::Regular(c) => name.push(char::from(c)),
                _ => break,
            }
        }
        Name(name)
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
pub struct Numeric(pub u32);

impl TryFrom<&mut Iter<'_, u8>> for Numeric {
    type Error = &'static str;

    fn try_from(value: &mut Iter<'_, u8>) -> Result<Self, Self::Error> {
        let mut numeric: u32 = 0;
        loop {
            let curr = match value.next() {
                Some(e) => e,
                None => break,
            };
            match CharacterSet::from(curr) {
                CharacterSet::Regular(b'+' | b'-') => (),
                CharacterSet::Regular(
                    b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9',
                ) => numeric = numeric * 10 + char::from(*curr).to_digit(10).unwrap(),
                _ => break,
            }
        }
        Ok(Self(numeric))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct IndirectObject {
    pub obj_num: Numeric,
    pub obj_gen: Numeric,
    pub is_reference: bool,
}

impl TryFrom<&mut PdfBytes<'_>> for IndirectObject {
    type Error = &'static str;

    // Read bytes b"1 0 R: to IndirectRef
    fn try_from(byte: &mut PdfBytes<'_>) -> Result<Self, &'static str> {
        let obj_num = match byte.next() {
            Some(Token::Numeric(n)) => Numeric(n),
            Some(t) => return Err("Unable to read components of indirect object; found incorrect first token"),
            _ => return Err("Unable to read first component of indirect object"),
        };
        let obj_gen = match byte.next() {
            Some(Token::Numeric(n)) => Numeric(n),
            Some(t) => return Err("Unable to read components of indirect object; found incorrect second token"),
            _ => return Err("Unable to read second component of indirect object"),
        };
        let is_reference = match byte.next() {
            Some(Token::String(b"R")) => true,
            Some(Token::String(b"obj")) => false,
            Some(c) => return Err("Incoherent character found in third component of indirect object"),
            None => return Err("Unable to read third component of indirect object"),
        };
        Ok(IndirectObject {
            obj_num,
            obj_gen,
            is_reference
        })
    }
}

impl From<&mut Iter<'_, u8>> for IndirectObject {
    // Read bytes b"1 0 R: to IndirectRef
    fn from(byte: &mut Iter<'_, u8>) -> Self {
        let obj_num = Numeric::try_from(&mut *byte).unwrap();
        let obj_gen = Numeric::try_from(&mut *byte).unwrap();
        let is_reference = match byte.next() {
            Some(b'R') => true,
            Some(b'o') => {
                byte.next().unwrap();
                byte.next().unwrap();
                false
            }
            Some(c) => {
                panic!("Incoherent character found in third component of indirect object: {c}")
            }
            None => panic!("Unable to read third component of indirect object"),
        };
        byte.next(); // TODO: check whitespace
        IndirectObject {
            obj_num,
            obj_gen,
            is_reference,
        }
    }
}

// extract trailer dictionnary
#[derive(Debug, PartialEq)]
pub struct Trailer {
    size: Numeric,
    prev: Option<Numeric>,
    pub root: IndirectObject,         // Catalogue dictionnary
    encrypt: Option<IndirectObject>,  // Encryption dictionnary
    pub info: Option<IndirectObject>, // Information dictionary
    id: Option<Vec<String>>,          // An array of two byte-strings constituting a file identifier
}

impl From<&[u8]> for Trailer {
    fn from(bytes: &[u8]) -> Self {
        let mut size = Numeric(9999);
        let mut root = IndirectObject {
            obj_num: Numeric(0),
            obj_gen: Numeric(0),
            is_reference: true,
        };
        let mut info = None;
        let id = None;
        let mut prev = None;
        let mut encrypt = None;

        let mut iter = bytes.iter();
        assert_eq!(*iter.next().unwrap(), b'<');
        assert_eq!(*iter.next().unwrap(), b'<');

        while let Ok(name) = Name::try_from(&mut iter) {
            match name.0.as_str() {
                "Size" => size = Numeric::try_from(&mut iter).unwrap(),
                "Root" => root = IndirectObject::try_from(&mut iter).unwrap(),
                "Info" => info = IndirectObject::try_from(&mut iter).ok(),
                "Prev" => prev = Numeric::try_from(&mut iter).ok(),
                "Encrypt" => encrypt = IndirectObject::try_from(&mut iter).ok(),
                "ID" => (), //id = Array::try_from(iter).ok(),
                a => panic!("Unexpected key was found in trailer {a}"),
            };
        }

        Trailer {
            size,
            prev,
            root,
            encrypt,
            info,
            id,
        }
    }
}

type Rectangle = [Numeric; 4];
type Stream<'a> = &'a [u8];

#[derive(Debug)]
pub enum PageTreeKids {
    Page(Page),
    PageTreeNode(PageTreeNode),
}

impl PageTreeKids {

    fn new(bytes: &[u8], xref: &XrefTable) -> PageTreeKids {

        // Read header of dictionary
        let mut pdf = PdfBytes::new(bytes);

        println!("PageTreeKids bytes: {:?}", std::str::from_utf8(bytes));

        // Consume object header
        IndirectObject::try_from(&mut pdf);

        match pdf.next() {
            Some(Token::DictBegin) => (), // Ok continue
            Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
            None => panic!("PageTreeNodeRoot should be a dictionnary"),
        };

        // check type of kid
        while let Some(t) = pdf.next() {
            match t {
                Token::Name("Type") => match pdf.next() {
                    Some(Token::Name("Pages")) => return PageTreeKids::PageTreeNode(PageTreeNode::new(&bytes, &xref)),
                    Some(Token::Name("Page")) => return PageTreeKids::Page(Page::new(&bytes, &xref)),
                    Some(t) => panic!("Unexpected dictionnary type; found token {t:?}"),
                    None => panic!("Unexpected dictionnary type"),
                },
                Token::DictEnd => break,
                a => panic!("Unexpected key was found in dictionnary catalog {a:?}"),
            }
        };
        panic!("PageTreeKid should have a Type key");
    }
}

enum PageTreeParent {
    PageTreeNodeRoot(PageTreeNodeRoot),
    PageTreeNode(PageTreeNode),
}

#[derive(Debug)]
pub struct PageTreeNodeRoot {
    kids: Vec<PageTreeKids>, // PageTreeNode kids can be a Page or a PageTreeNode
    count: Numeric, // Number of leaf nodes
    // Inheritables (cf page 149)
    rotate: Option<Numeric>, // Number of degrees by which the page should be rotated clockwise when displayeds
    crop_box: Option<Rectangle>, // Rectangle
    media_box: Option<Rectangle>, // Rectangle
    resources: Option<IndirectObject>, // Resource dictionary
}

impl PageTreeNodeRoot {

    pub fn new(bytes: &[u8], xref: &XrefTable) -> Self {
        let mut pdf = PdfBytes::new(bytes);

        println!("PageTreeNodeRoot bytes: {:?}", std::str::from_utf8(bytes));

        // Consume object header
        IndirectObject::try_from(&mut pdf);

        match pdf.next() {
            Some(Token::DictBegin) => (), // Ok continue
            Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
            None => panic!("PageTreeNodeRoot should be a dictionnary"),
        };

        let mut kids = Vec::new();
        let mut count = Numeric(0);
        let mut rotate = None;
        let mut crop_box = None;
        let mut media_box = None;
        let mut resources = None;

        while let Some(t) = pdf.next() {
            match t {
                // check if the PageTreeNodeRoot dictionnary is of type Pages
                Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Pages"))),
                // array of indirect references to the immediate children of this node
                Token::Name("Kids") => {
                    assert_eq!(pdf.next(), Some(Token::ArrayBegin));
                    while let Ok(indirect_ref) = IndirectObject::try_from(&mut pdf) {
                        let kids_idx = xref.get(&indirect_ref).unwrap();
                        kids.push(PageTreeKids::new(&bytes[*kids_idx..], &xref));
                    }
                }
                Token::Name("Count") => {
                    count = match pdf.next() {
                        Some(Token::Numeric(n)) => Numeric(n),
                        Some(t) => panic!("Count should be a numeric; found {t:?}"),
                        None => panic!("Count should be a numeric"),
                    };
                }
                Token::Name("Rotate") => {
                    rotate = match pdf.next() {
                        Some(Token::Numeric(n)) => Some(Numeric(n)),
                        Some(t) => panic!("Rotate should be a numeric; found {t:?}"),
                        None => panic!("Rotate should be a numeric"),
                    };
                }
                Token::Name("CropBox") => {
                    assert_eq!(pdf.next(), Some(Token::ArrayBegin));
                    let mut crop_box_buff = [Numeric(0); 4];
                    for i in 0..4 {
                        crop_box_buff[i] = match pdf.next() {
                            Some(Token::Numeric(n)) => Numeric(n),
                            Some(t) => panic!("CropBox should be a numeric; found {t:?}"),
                            None => panic!("CropBox should be a numeric"),
                        }
                    }
                    assert_eq!(pdf.next(), Some(Token::ArrayEnd));
                    crop_box = Some(crop_box_buff);
                }
                Token::Name("MediaBox") => {
                    assert_eq!(pdf.next(), Some(Token::ArrayBegin));
                    let mut media_box_buff = [Numeric(0); 4];
                    for i in 0..4 {
                        media_box_buff[i] = match pdf.next() {
                            Some(Token::Numeric(n)) => Numeric(n),
                            Some(t) => panic!("MediaBox should be a numeric; found {t:?}"),
                            None => panic!("MediaBox should be a numeric"),
                        }
                    }
                    assert_eq!(pdf.next(), Some(Token::ArrayEnd));
                    media_box = Some(media_box_buff);
                }
                Token::Name("Resources") => {
                    resources = Some(IndirectObject::try_from(&mut pdf).unwrap());
                }
                Token::DictEnd => break,
                a => panic!("Unexpected key was found in dictionnary page tree root node {a:?}"),
            };
        }
        PageTreeNodeRoot { kids, count, rotate, crop_box, media_box, resources }
    }
}

#[derive(Debug)]
pub struct PageTreeNode {
    // parent: PageTreeParent<'a>, // The page tree node's parent
    kids: Vec<PageTreeKids>, // PageTreeNode kids can be a Page or a PageTreeNode
    count: Numeric, // Number of leaf nodes
}

impl PageTreeNode {
    fn new(bytes: &[u8], xref: &XrefTable) -> Self {
        let mut pdf = PdfBytes::new(bytes);

        // Consume object header
        IndirectObject::try_from(&mut pdf);

        match pdf.next() {
            Some(Token::DictBegin) => (), // Ok continue
            Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
            None => panic!("PageTreeNodeRoot should be a dictionnary"),
        };

        let mut kids = Vec::new();
        let mut count = Numeric(0);

        while let Some(t) = pdf.next() {
            match t {
                // check if the PageTreeNodeRoot dictionnary is of type Pages
                Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Pages"))),
                // array of indirect references to the immediate children of this node
                Token::Name("Kids") => {
                    assert_eq!(pdf.next(), Some(Token::ArrayBegin));
                    while let Ok(indirect_ref) = IndirectObject::try_from(&mut pdf) {
                        let kids_idx = xref.get(&indirect_ref).unwrap();
                        kids.push(PageTreeKids::new(&bytes[*kids_idx..], &xref));
                    }
                }
                Token::Name("Count") => {
                    count = match pdf.next() {
                        Some(Token::Numeric(n)) => Numeric(n),
                        Some(t) => panic!("Count should be a numeric; found {t:?}"),
                        None => panic!("Count should be a numeric"),
                    };
                }
                Token::DictEnd => break,
                a => panic!("Unexpected key was found in dictionnary catalog {a:?}"),
            };
        }
        PageTreeNode { kids, count }
    }
}

#[derive(Debug)]
struct Page {
    // parent: PageTreeParent<'a>, // The page tree node's parent
    last_modified: Option<String>, // Date and time of last modification
    resources: IndirectObject,     // Resource dictionary
    media_box: Rectangle,          //rectangle
    // crop_box: Option<Rectangle>,   //rectangle
    // bleed_box: Option<Rectangle>,  //rectangle
    // trim_box: Option<Rectangle>,   //rectangle
    // art_box: Option<Rectangle>,    //rectangle
    // box_color_info: Option<IndirectObject>, // Box color information dictionary
    // contents: Option<Stream<'a>>,  // Content stream; if None Page is empty
    // rotate: Option<Numeric>,
    // group: Option<IndirectObject>, // Group attributes dictionary
    // thumb: Option<Stream<'a>>,
    // b: Option<Vec<IndirectObject>>, // array of indirect references to article beads
    // dur: Option<Numeric>,           // page's display duration
    // trans: Option<IndirectObject>,  // transition dictionary
    // annots: Option<Vec<IndirectObject>>, // array of annotation dictionaries
    // aa: Option<IndirectObject>,     // additional actions dictionary
    // metadata: Option<Stream<'a>>,   // metadata stream of the page
    // piece_info: Option<IndirectObject>, // piece information dictionary
    // struct_parents: Option<Numeric>, // integer
    // id: Option<String>,             // byte string
    // pz: Option<Numeric>,            // integer
    // separation_info: Option<IndirectObject>, // separation information dictionary
    // tabs: Option<Name>, // name specifying the tab order to be used for annotations on the page
    // template_instantiated: Option<Name>, // template dictionary
    // pres_steps: Option<IndirectObject>, // navigation node dictionary
    // user_unit: Option<Numeric>, // number specifying the size of default user space units
    // vp: Option<IndirectObject>, // array of numbers specifying the page's viewport
}

impl Page {
    fn new(bytes: &[u8], xref: &XrefTable) -> Self {
        let mut pdf = PdfBytes::new(bytes);

        // Consume object header
        IndirectObject::try_from(&mut pdf);

        // Consume <<
        match pdf.next() {
            Some(Token::DictBegin) => (), // Ok continue
            Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
            None => panic!("PageTreeNodeRoot should be a dictionnary"),
        };

        let mut last_modified = None;
        let mut resources = None;
        let mut media_box = None;

        while let Some(t) = pdf.next() {
            match t {
                // Check if the Page dictionnary is of type Page
                Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Page"))),

                // Last modified date of the page
                Token::Name("LastModified") => {
                    match pdf.next() {
                        Some(Token::LitteralString(s)) => {
                            last_modified = Some(String::from(std::str::from_utf8(s).unwrap()));
                        }
                        Some(t) => panic!("LastModified should be a string; found {t:?}"),
                        None => panic!("LastModified should be a string"),
                    }
                }

                // Resource dictionnary
                Token::Name("Resources") => {
                    resources = Some(IndirectObject::try_from(&mut pdf).unwrap());
                }

                // Media box reactangle
                Token::Name("MediaBox") => {
                    assert_eq!(pdf.next(), Some(Token::ArrayBegin));
                    let mut media_box_buff = [Numeric(0); 4];
                    for i in 0..4 {
                        media_box_buff[i] = match pdf.next() {
                            Some(Token::Numeric(n)) => Numeric(n),
                            Some(t) => panic!("MediaBox should be a numeric; found {t:?}"),
                            None => panic!("MediaBox should be a numeric"),
                        }
                    }
                    assert_eq!(pdf.next(), Some(Token::ArrayEnd));
                    media_box = Some(media_box_buff);
                }
                Token::DictEnd => break,
                _ => (),
            };
        }
        Page { last_modified, resources: resources.unwrap(), media_box: media_box.unwrap() }
    }
}

#[derive(Debug, PartialEq)]
// Defined in page 139;  commented is to be implemented
pub struct Catalog {
    pub pages: Option<IndirectObject>, // The page tree node that is the root of the document’s page tree
}

impl From<&[u8]> for Catalog {
    fn from(bytes: &[u8]) -> Self {
        let mut pdf = PdfBytes::new(bytes);
        // Consume object header
        IndirectObject::try_from(&mut pdf);

        match pdf.next() {
            Some(Token::DictBegin) => (),
            Some(t) => panic!("Catalog should be a dictionnary; found {t:?}"),
            None => panic!("Catalog should be a dictionnary"),
        };

        let mut pages = None;

        while let Some(t) = pdf.next() {
            match t {
                Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Catalog"))),
                Token::Name("Pages") => {
                    pages = Some(IndirectObject::try_from(&mut pdf).unwrap());
                }
                Token::DictEnd => break,
                a => panic!("Unexpected key was found in dictionnary catalog {a:?}"),
            };
        }
        Catalog { pages }
    }
}

#[derive(Debug, PartialEq)]
pub struct Info<'a> {
    title: Option<&'a str>,
    author: Option<&'a str>,
    creator: Option<&'a str>,
    producer: Option<&'a str>,
    creation_date: Option<&'a str>,
    mod_date: Option<&'a str>,
}

impl<'a> From<&'a [u8]> for Info<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        let mut pdf = PdfBytes::new(bytes);

        // Consume object header
        IndirectObject::try_from(&mut pdf).unwrap();

        match pdf.next() {
            Some(Token::DictBegin) => (),
            Some(t) => panic!("Info should be a dictionnary; found {t:?}"),
            None => panic!("Info should be a dictionnary"),
        };

        let mut title = None;
        let mut author = None;
        let mut creator = None;
        let mut producer = None;
        let mut creation_date = None;
        let mut mod_date = None;

        while let Some(t) = pdf.next() {
            match t {
                Token::Name("Title") => match pdf.next() {
                    Some(Token::LitteralString(s)) => title = std::str::from_utf8(s).ok(),
                    Some(t) => panic!("Title should be a string; found {t:?}"),
                    _ => panic!("Title should be a string"),
                },
                Token::Name("Author") => match pdf.next() {
                    Some(Token::LitteralString(s)) => author = std::str::from_utf8(s).ok(),
                    _ => panic!("Author should be a string"),
                },
                Token::Name("Creator") => match pdf.next() {
                    Some(Token::LitteralString(s)) => creator = std::str::from_utf8(s).ok(),
                    _ => panic!("Creator should be a string"),
                },
                Token::Name("Producer") => match pdf.next() {
                    Some(Token::LitteralString(s)) => producer = std::str::from_utf8(s).ok(),
                    Some(t) => panic!("Producer should be a string; found {t:?}"),
                    _ => panic!("Producer should be a string"),
                },
                Token::Name("CreationDate") => match pdf.next() {
                    Some(Token::LitteralString(s)) => creation_date = std::str::from_utf8(s).ok(),
                    _ => panic!("CreationDate should be a string"),
                },
                Token::Name("ModDate") => match pdf.next() {
                    Some(Token::LitteralString(s)) => mod_date = std::str::from_utf8(s).ok(),
                    _ => panic!("Modification date should be a string"),
                },
                Token::Name("PTEX.Fullbanner") => {
                    pdf.next();
                }
                Token::Name(n) => println!("Key {:?} is not implemented", n),
                Token::DictEnd => break,
                t => panic!("Unexpected key was found in info dictionnary {t:?}"),
            };
        }
        Info {
            title,
            author,
            creator,
            producer,
            creation_date,
            mod_date,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn read_name_object_from_u8() {
        let entry_sample = b"/Type /Font".as_slice();
        assert_eq!(Name::from(entry_sample), Name(String::from("Type")));
    }

    #[test]
    fn read_name_object_from_u8_2() {
        let entry_sample = b"/Root".as_slice();
        assert_eq!(Name::from(entry_sample), Name(String::from("Root")));
    }

    #[test]
    fn read_numeric_object() {
        let mut entry_sample = b"6".iter();
        assert_eq!(Numeric::try_from(&mut entry_sample), Ok(Numeric(6)));
    }

    #[test]
    fn read_numeric_object_with_sign() {
        let mut entry_sample = b"+54".iter();
        assert_eq!(Numeric::try_from(&mut entry_sample), Ok(Numeric(54)));
    }

    #[test]
    fn read_indirect_object_ref() {
        let mut object_ref_sample = b"1 0 R".iter();
        assert_eq!(
            IndirectObject::from(&mut object_ref_sample),
            IndirectObject {
                obj_num: Numeric(1),
                obj_gen: Numeric(0),
                is_reference: true,
            }
        );
    }

    #[test]
    fn read_trailer_multi_lines() {
        let dict = b"<<\n  /Size 6\n  /Root 1 0 R\n>>".as_slice();
        assert_eq!(
            Trailer::from(dict),
            Trailer {
                size: Numeric(6),
                root: IndirectObject {
                    obj_num: Numeric(1),
                    obj_gen: Numeric(0),
                    is_reference: true
                },
                info: None,
                prev: None,
                encrypt: None,
                id: None
            }
        );
    }

    #[test]
    fn read_trailer_from_one_line() {
        let dict =
            b"<< /Size 26 /Root 13 0 R /Info 1 0 R /ID [ <4e949515aaf132498f650e7bde6cdc2f>\n<4e949515aaf132498f650e7bde6cdc2f> ] >>"
                .as_slice();
        assert_eq!(
            Trailer::from(dict),
            Trailer {
                size: Numeric(26),
                root: IndirectObject {
                    obj_num: Numeric(13),
                    obj_gen: Numeric(0),
                    is_reference: true
                },
                info: Some(IndirectObject {
                    obj_num: Numeric(1),
                    obj_gen: Numeric(0),
                    is_reference: true
                }),
                prev: None,
                encrypt: None,
                id: None
            }
        );
    }

    #[test]
    fn test_catalog() {
        let catalog = Catalog::from(b"1 0 obj  % entry point\n    <<\n      /Type /Catalog\n      /Pages 2 0 R\n    >>\n    endobj".as_slice());
        assert_eq!(
            catalog,
            Catalog {
                pages: Some(IndirectObject {
                    obj_num: Numeric(2),
                    obj_gen: Numeric(0),
                    is_reference: true
                })
            }
        )
    }

    #[test]
    fn test_info_dict_1() {
        let info_object = b"1 0 obj\n<< /Title (sample) /Author (Philip Hutchison) /Creator (Pages) /Producer (Mac OS X 10.5.4 Quartz PDFContext)\n/CreationDate (D:20080701052447Z00'00') /ModDate (D:20080701052447Z00'00')\n>>\nendobj";
        let info = Info::from(info_object.as_slice());
        assert_eq!(
            info,
            Info {
                title: Some("sample"),
                author: Some("Philip Hutchison"),
                creator: Some("Pages"),
                producer: Some("Mac OS X 10.5.4 Quartz PDFContext"),
                creation_date: Some("D:20080701052447Z00'00'"),
                mod_date: Some("D:20080701052447Z00'00'")
            }
        );
    }
}
