use core::panic;
use std::{collections::HashMap, fmt::Display};

use crate::{
    cmap::ToUnicodeCMap,
    filters::flate_decode,
    object::{Dictionary, Name, Number, Object},
    text,
    xref::XrefTable,
    Extract,
};

use crate::object::Stream as StreamObject;

type Rectangle = [Number; 4];
#[derive(Debug, PartialEq)]
enum Filter {
    FlateDecode,
}

impl From<Name> for Filter {
    fn from(value: Name) -> Self {
        match value.as_str() {
            "FlateDecode" => Filter::FlateDecode,
            filter => panic!("Filter name {filter:?} is currently not supported"),
        }
    }
}

#[derive(Debug, PartialEq)]
struct StreamDictionary {
    length: Number,
    filter: Option<Filter>,
}

impl From<Dictionary<'_>> for StreamDictionary {
    fn from(value: Dictionary) -> Self {
        StreamDictionary {
            length: match value.get("Length").unwrap() {
                Object::Numeric(n) => n.clone(),
                Object::Ref((obj, gen), xref, bytes) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => match Object::new(bytes, address, xref) {
                            Object::Numeric(n) => n,
                            _ => panic!("Length should be a numeric"),
                        },
                        None => panic!("Length should be an indirect object"),
                    }
                }
                _ => panic!("Length should be a numeric"),
            },
            filter: match value.get("Filter") {
                Some(Object::Name(name)) => Some(Filter::from(name.clone())),
                None => None,
                _ => panic!("Filter should be a name"),
            },
        }
    }
}

type StreamContent = Vec<u8>;

#[derive(Debug, PartialEq)]
struct Stream(StreamDictionary, StreamContent);

impl Stream {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        let (dict, stream) = match Object::new(bytes, curr_idx, xref) {
            Object::Stream(StreamObject { header, bytes }) => {
                (StreamDictionary::from(header), bytes)
            }
            _ => panic!("Stream should be a dictionary"),
        };
        Stream(dict, stream)
    }

    pub fn get_data(&self) -> String {
        match &self.0.filter {
            Some(Filter::FlateDecode) => flate_decode(&self.1),
            // Some(f) => panic!("Filter {f:?} is not supported at the moment"),
            None => std::str::from_utf8(&self.1).unwrap().to_string(),
        }
    }
}

impl From<StreamObject<'_>> for Stream {
    fn from(object: StreamObject<'_>) -> Self {
        Stream(StreamDictionary::from(object.header), object.bytes)
    }
}

#[derive(Debug, PartialEq)]
pub enum PageTreeKids {
    Page(Page),
    PageTreeNode(PageTreeNode),
}

impl PageTreeKids {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => match dict.get("Type") {
                Some(Object::Name(name)) => match name.as_str() {
                    "Pages" => PageTreeKids::PageTreeNode(PageTreeNode::new(bytes, curr_idx, xref)),
                    "Page" => PageTreeKids::Page(Page::new(bytes, curr_idx, xref)),
                    _ => panic!("Unexpected dictionnary type"),
                },
                Some(o) => panic!("Type should be a name, found object {o:?}"),
                None => panic!("Type was not found in dictionnary, {dict:?}"),
            },
            _ => panic!("PageTreeKids should be a dictionary"),
        }
    }

    pub fn extract(&self, e: Extract) -> String {
        match self {
            PageTreeKids::Page(page) => page.extract(e),
            PageTreeKids::PageTreeNode(page_tree_node) => page_tree_node.extract(e),
        }
    }
}

#[derive(Debug, PartialEq)]
struct Font {
    subtype: Name,
    name: Option<Name>,
    base_font: Name,
    first_char: Option<Number>,
    last_char: Option<Number>,
    to_unicode: Option<ToUnicodeCMap>,
}

impl Display for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Subtype: {:?}\nName: {:?}\nBaseFont: {:?}\nFirstChar: {:?}\nLastChar: {:?}\nToUnicode: {:?}", self.subtype, self.name, self.base_font, self.first_char, self.last_char, self.to_unicode
        )
    }
}

impl From<Dictionary<'_>> for Font {
    fn from(value: Dictionary) -> Self {
        match value.get("Type") {
            Some(Object::Name(t)) => {
                if t != "Font" {
                    panic!("Font dictionnary 'Type' key should be 'Font', found {t:?}")
                }
            }
            Some(o) => panic!("Font dictionnary 'Type' key a Name object, found {o:?}"),
            None => panic!("Font dictionnary should have a 'Type' key"),
        };
        Font {
            subtype: match value.get("Subtype").unwrap() {
                Object::Name(name) => name.clone(),
                _ => panic!("Subtype should be a name"),
            },
            name: match value.get("Name") {
                Some(Object::Name(name)) => Some(name.clone()),
                Some(o) => panic!("Name should be a name, found {o:?}"),
                None => None,
            },
            base_font: match value.get("BaseFont").unwrap() {
                Object::Name(name) => name.clone(),
                _ => panic!("BaseFont should be a name"),
            },
            first_char: match value.get("FirstChar") {
                Some(Object::Numeric(n)) => Some(n.clone()),
                Some(o) => panic!("FirstChar should be a numeric object, found {o:?}"),
                None => None,
            },
            last_char: match value.get("LastChar") {
                Some(Object::Numeric(n)) => Some(n.clone()),
                Some(o) => panic!("LastChar should be a numeric object, found {o:?}"),
                None => None,
            },
            to_unicode: match value.get("ToUnicode") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => match Object::new(bytes, address, xref) {
                            Object::Stream(stream) => {
                                Some(ToUnicodeCMap::from(Stream::from(stream).get_data()))
                            }
                            o => panic!("ToUnicode should be a stream object, found {o:?}"),
                        },
                        None => panic!("ToUnicode stream object not found in xref table"),
                    }
                }
                None => None,
                _ => panic!("ToUnicode should be an indirect object"),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
struct FontMap(HashMap<Name, Font>);

impl From<Dictionary<'_>> for FontMap {
    fn from(value: Dictionary) -> Self {
        FontMap(
            value
                .iter()
                .map(|(key, value)| match value {
                    Object::Ref((obj, gen), xref, bytes) => match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => {
                            match Object::new(bytes, address, xref) {
                            Object::Dictionary(t) => (key.clone(), Font::from(t)),
                            o => panic!("Font object is not a dictionary, found {o:?}"),
                        }},
                        None => panic!("Font dictionnary object associated to {key:?} was not found in xref table"),
                    },
                    _ => panic!("Font should be an indirect object"),
                })
                .collect(),
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct Resources {
    font: Option<FontMap>,
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
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    xref.get_and_fix(&(*obj, *gen), bytes).map(|address| {
                        FontMap::from(match Object::new(bytes, address, xref) {
                            Object::Dictionary(t) => t,
                            _ => panic!("Font should be a dictionary"),
                        })
                    })
                }
                Some(Object::Dictionary(t)) => Some(FontMap::from(t.clone())),
                None => None,
                f => panic!("Font should be an indirect object or a dictionary; found {f:?}"),
            },
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PageTreeNodeRoot {
    kids: Vec<PageTreeKids>, // PageTreeNode kids can be a Page or a PageTreeNode
    count: Number,           // Number of leaf nodes
    // Inheritables (cf page 149)
    rotate: Option<Number>, // Number of degrees by which the page should be rotated clockwise when displayeds
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

    pub fn extract(&self, e: Extract) -> String {
        self.kids
            .iter()
            .map(|kid| kid.extract(e.clone()))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl From<Dictionary<'_>> for PageTreeNodeRoot {
    fn from(value: Dictionary) -> Self {
        PageTreeNodeRoot {
            kids: match value.get("Kids").unwrap() {
                Object::Array(arr) => arr
                    .iter()
                    .map(|kid| match kid {
                        Object::Ref((obj, gen), xref, bytes) => {
                            match xref.get_and_fix(&(*obj, *gen), bytes) {
                                Some(address) => PageTreeKids::new(bytes, address, xref),
                                None => panic!("Kid not found in xref table"),
                            }
                        }
                        _ => panic!("Kid should be an indirect object"),
                    })
                    .collect(),
                _ => panic!("Kids should be an array"),
            },
            count: match value.get("Count").unwrap() {
                Object::Numeric(n) => n.clone(),
                _ => panic!("Count should be a numeric"),
            },
            rotate: match value.get("Rotate") {
                Some(Object::Numeric(n)) => Some(n.clone()),
                None => None,
                _ => panic!("Rotate should be a numeric"),
            },
            crop_box: match value.get("CropBox") {
                Some(Object::Array(arr)) => Some([
                    match &arr[0] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                    match &arr[1] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                    match &arr[2] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                    match &arr[3] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("CropBox should be an array of numeric"),
                    },
                ]),
                None => None,
                _ => panic!("CropBox should be an array"),
            },
            media_box: match value.get("MediaBox") {
                Some(Object::Array(arr)) => Some([
                    match &arr[0] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match &arr[1] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match &arr[2] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                    match &arr[3] {
                        Object::Numeric(n) => n.clone(),
                        _ => panic!("MediaBox should be an array of numeric"),
                    },
                ]),
                None => None,
                _ => panic!("MediaBox should be an array"),
            },
            resources: match value.get("Resources") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Some(Resources::new(bytes, address, xref)),
                        None => panic!("Kid not found in xref table"),
                    }
                }
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
    count: Number,           // Number of leaf nodes
}

impl PageTreeNode {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }

    pub fn extract(&self, e: Extract) -> String {
        self.kids
            .iter()
            .map(|kid| kid.extract(e.clone()))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl From<Dictionary<'_>> for PageTreeNode {
    fn from(value: Dictionary) -> Self {
        PageTreeNode {
            kids: match value.get("Kids").unwrap() {
                Object::Array(arr) => arr
                    .iter()
                    .map(|kid| match kid {
                        Object::Ref((obj, gen), xref, bytes) => {
                            match xref.get_and_fix(&(*obj, *gen), bytes) {
                                Some(address) => PageTreeKids::new(bytes, address, xref),
                                None => panic!("Kid not found in xref table"),
                            }
                        }
                        _ => panic!("Kid should be an indirect object"),
                    })
                    .collect(),
                _ => panic!("Kids should be an array"),
            },
            count: match value.get("Count").unwrap() {
                Object::Numeric(n) => n.clone(),
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
    contents: Option<Stream>,
}

impl Page {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: &XrefTable) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }

    pub fn extract(&self, e: Extract) -> String {
        match e {
            Extract::Text => self.extract_text(),
            Extract::RawContent => self.extract_stream(),
            Extract::Font => self.extract_font(),
        }
    }

    pub fn extract_font(&self) -> String {
        match &self.resources.font {
            Some(font_map) => font_map
                .0
                .values()
                .map(|font| format!("{font}\n"))
                .collect::<Vec<String>>()
                .join("\n"),
            None => panic!("Font should not be empty"),
        }
    }

    pub fn extract_text(&self) -> String {
        text::StreamContent::from(self.extract_stream().as_bytes()).get_text()
    }

    pub fn extract_stream(&self) -> String {
        // Extract text
        match &self.contents {
            Some(stream) => stream.get_data(),
            None => panic!("Contents should not be empty"),
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
                Object::Ref((obj, gen), xref, bytes) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Resources::new(bytes, address, xref),
                        None => panic!("Resource dictionnary address not found in xref keys"),
                    }
                }
                t => panic!("Resources should be an dictionary object {t:?}"),
            },
            media_box: match value.get("MediaBox") {
                Some(Object::Array(arr)) => Some([
                    match &arr[0] {
                        Object::Numeric(n) => n.clone(),
                        o => panic!("MediaBox should be an array of numeric, found {o:?}"),
                    },
                    match &arr[1] {
                        Object::Numeric(n) => n.clone(),
                        o => panic!("MediaBox should be an array of numeric, found {o:?}"),
                    },
                    match &arr[2] {
                        Object::Numeric(n) => n.clone(),
                        o => panic!("MediaBox should be an array of numeric, found {o:?}"),
                    },
                    match &arr[3] {
                        Object::Numeric(n) => n.clone(),
                        o => panic!("MediaBox should be an array of numeric, found {o:?}"),
                    },
                ]),
                Some(a) => panic!("MediaBox should be an array; found {a:?}"),
                None => None,
            },
            contents: match value.get("Contents") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Some(Stream::new(bytes, address, xref)),
                        None => panic!("Resource dictionnary address not found in xref keys"),
                    }
                }
                None => None,
                _ => panic!("Contents should be an indirect object"),
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

    pub fn extract(&self, e: Extract) -> String {
        match &self.pages {
            Some(page_tree_node) => page_tree_node.extract(e),
            None => panic!("Pages should not be empty"),
        }
    }
}

impl From<Dictionary<'_>> for Catalog {
    fn from(value: Dictionary) -> Self {
        Catalog {
            pages: match value.get("Pages").unwrap() {
                Object::Ref((obj, gen), xref, bytes) => xref
                    .get_and_fix(&(*obj, *gen), bytes)
                    .map(|address| PageTreeNodeRoot::new(bytes, address, xref)),
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
