use std::collections::HashMap;

use crate::{
    object::{Dictionary, IndirectObject, Name, Numeric, Object},
    xref::XrefTable,
};

type Rectangle = [Numeric; 4];

#[derive(Debug, PartialEq)]
pub enum PageTreeKids {
    Page(Page),
    PageTreeNode(PageTreeNode),
}

impl PageTreeKids {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => match dict.get("Type").unwrap() {
                Object::Name(name) => match name.as_str() {
                    "Pages" => PageTreeKids::PageTreeNode(PageTreeNode::new(bytes, curr_idx, xref)),
                    "Page" => PageTreeKids::Page(Page::new(bytes, curr_idx, xref)),
                    _ => panic!("Unexpected dictionnary type"),
                },
                _ => panic!("Type should be a name"),
            },
            _ => panic!("PageTreeKids should be a dictionary"),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Font(HashMap<Name, IndirectObject>);

impl From<Dictionary<'_>> for Font {
    fn from(value: Dictionary) -> Self {
        Font(
            value
                .iter()
                .map(|(key, value)| match value {
                    Object::Ref((obj, gen), _xref, _bytes) => (key.clone(), (*obj, *gen)),
                    _ => panic!("Font should be an indirect object"),
                })
                .collect(),
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct Resources {
    font: Option<Font>,
}

impl Resources {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary<'_>> for Resources {
    fn from(value: Dictionary) -> Self {
        Resources {
            font: match value.get("Font") {
                Some(Object::Dictionary(t)) => Some(Font::from(t.clone())),
                None => None,
                _ => panic!("Font should be an indirect object"),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PageTreeNodeRoot {
    kids: Vec<PageTreeKids>, // PageTreeNode kids can be a Page or a PageTreeNode
    count: Numeric,          // Number of leaf nodes
    // Inheritables (cf page 149)
    rotate: Option<Numeric>, // Number of degrees by which the page should be rotated clockwise when displayeds
    crop_box: Option<Rectangle>, // Rectangle
    media_box: Option<Rectangle>, // Rectangle
    resources: Option<Resources>, // Resource dictionary
}

impl PageTreeNodeRoot {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary<'_>> for PageTreeNodeRoot {
    fn from(value: Dictionary) -> Self {
        PageTreeNodeRoot {
            kids: match value.get("Kids").unwrap() {
                Object::Array(arr) => arr
                    .iter()
                    .map(|kid| match kid {
                        Object::Ref((obj, gen), xref, bytes) => match xref.get(&(*obj, *gen)) {
                            Some(address) => PageTreeKids::new(&bytes, *address, xref),
                            None => panic!("Kid not found in xref table"),
                        },
                        _ => panic!("Kid should be an indirect object"),
                    })
                    .collect(),
                _ => panic!("Kids should be an array"),
            },
            count: match value.get("Count").unwrap() {
                Object::Numeric(n) => *n,
                _ => panic!("Count should be a numeric"),
            },
            rotate: match value.get("Rotate") {
                Some(Object::Numeric(n)) => Some(*n),
                None => None,
                _ => panic!("Rotate should be a numeric"),
            },
            crop_box: match value.get("CropBox") {
                Some(Object::Array(arr)) => Some([
                    match arr[0] {
                        Object::Numeric(n) => n,
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                    match arr[1] {
                        Object::Numeric(n) => n,
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                    match arr[2] {
                        Object::Numeric(n) => n,
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                    match arr[3] {
                        Object::Numeric(n) => n,
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                ]),
                None => None,
                _ => panic!("CropBox should be an array"),
            },
            media_box: match value.get("MediaBox") {
                Some(Object::Array(arr)) => Some([
                    match arr[0] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match arr[1] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match arr[2] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match arr[3] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                ]),
                None => None,
                _ => panic!("MediaBox should be an array"),
            },
            resources: match value.get("Resources") {
                Some(Object::Ref((obj, gen), xref, bytes)) => match xref.get(&(*obj, *gen)) {
                    Some(address) => Some(Resources::new(&bytes, *address, xref)),
                    None => panic!("Kid not found in xref table"),
                },
                None => None,
                _ => panic!("Resources should be an indirect object"),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PageTreeNode {
    // parent: PageTreeParent<'a>, // The page tree node's parent
    kids: Vec<PageTreeKids>, // PageTreeNode kids can be a Page or a PageTreeNode
    count: Numeric,          // Number of leaf nodes
}

impl PageTreeNode {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary<'_>> for PageTreeNode {
    fn from(value: Dictionary) -> Self {
        PageTreeNode {
            kids: match value.get("Kids").unwrap() {
                Object::Array(arr) => arr
                    .iter()
                    .map(|kid| match kid {
                        Object::Ref((obj, gen), xref, bytes) => match xref.get(&(*obj, *gen)) {
                            Some(address) => PageTreeKids::new(&bytes, *address, xref),
                            None => panic!("Kid not found in xref table"),
                        },
                        _ => panic!("Kid should be an indirect object"),
                    })
                    .collect(),
                _ => panic!("Kids should be an array"),
            },
            count: match value.get("Count").unwrap() {
                Object::Numeric(n) => *n,
                _ => panic!("Count should be a numeric"),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Page {
    // parent: PageTreeParent<'a>, // The page tree node's parent
    last_modified: Option<String>, // Date and time of last modification
    resources: Resources,          // Resource dictionary
    media_box: Option<Rectangle>,  //rectangle
}

impl Page {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary<'_>> for Page {
    fn from(value: Dictionary) -> Self {
        Page {
            last_modified: match value.get("LastModified") {
                Some(Object::String(s)) => Some(s.clone()),
                None => None,
                _ => panic!("LastModified should be a string"),
            },
            resources: match value.get("Resources").unwrap() {
                Object::Dictionary(t) => Resources::from(t.clone()),
                Object::Ref((obj, gen), xref, bytes) => match xref.get(&(*obj, *gen)) {
                    Some(address) => Resources::new(&bytes, *address, xref),
                    None => panic!("Resource dictionnary address not found in xref keys"),
                },
                t => panic!("Resources should be an dictionary object {t:?}"),
            },
            media_box: match value.get("MediaBox") {
                Some(Object::Array(arr)) => Some([
                    match arr[0] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match arr[1] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match arr[2] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match arr[3] {
                        Object::Numeric(n) => n,
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                ]),
                Some(a) => panic!("MediaBox should be an array; found {a:?}"),
                None => None,
            },
        }
    }
}

// Document Catalog
// Defined in page 139;  commented is to be implemented
#[derive(Debug, PartialEq)]
pub struct Catalog {
    // The page tree node that is the root of the document’s page tree
    // Must be an indirect reference
    pub pages: Option<PageTreeNodeRoot>,
}

impl Catalog {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary<'_>> for Catalog {
    fn from(value: Dictionary) -> Self {
        Catalog {
            pages: match value.get("Pages").unwrap() {
                Object::Ref((obj, gen), xref, bytes) => match xref.get(&(*obj, *gen)) {
                    Some(address) => Some(PageTreeNodeRoot::new(&bytes, *address, xref)),
                    None => None,
                },
                _ => panic!("Pages should be an indirect object"),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_catalog() {
        let catalog = Catalog::new(b"1 0 obj  % entry point\n    <<\n      /Type /Catalog\n      /Pages 2 0 R\n    >>\n    endobj".as_slice(), 0, &XrefTable::new());
        assert_eq!(catalog, Catalog { pages: None })
    }
}
