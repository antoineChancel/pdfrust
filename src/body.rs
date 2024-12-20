use core::panic;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Display,
    rc::{Rc, Weak},
};

use crate::{
    algebra::Number,
    cmap::ToUnicodeCMap,
    content,
    filters::flate_decode,
    object::{Array, Dictionary, Name, Object},
    xref::XRef,
    Extract,
};

use crate::object::Stream as StreamObject;

#[derive(Debug, PartialEq)]
pub struct Rectangle([Number; 4]);

impl From<Array<'_>> for Rectangle {
    fn from(array: Array) -> Self {
        if array.len() != 4 {
            panic!("PDF rectangle contains 4 values, found {}", array.len())
        };
        let value: [Number; 4] = array
            .iter()
            .map(|x| match x {
                Object::Numeric(n) => n.clone(),
                o => panic!("PDF rectangle values are numbers, found {o:?}"),
            })
            .collect::<Vec<Number>>()
            .try_into()
            .unwrap();
        Rectangle(value)
    }
}

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
                        Some(address) => match Object::new(bytes, address, xref.clone()) {
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
pub struct Stream(StreamDictionary, StreamContent);

impl Stream {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: Rc<XRef>) -> Self {
        let (dict, stream) = match Object::new(bytes, curr_idx, xref) {
            Object::Stream(StreamObject { header, bytes }) => {
                (StreamDictionary::from(header), bytes)
            }
            _ => panic!("Stream should be a dictionary"),
        };
        Stream(dict, stream)
    }

    pub fn get_data(&self) -> Vec<u8> {
        match &self.0.filter {
            Some(Filter::FlateDecode) => flate_decode(&self.1),
            // Some(f) => panic!("Filter {f:?} is not supported at the moment"),
            None => self.1.clone(),
        }
    }
}

impl From<StreamObject<'_>> for Stream {
    fn from(object: StreamObject<'_>) -> Self {
        Stream(StreamDictionary::from(object.header), object.bytes)
    }
}

#[derive(Debug)]
pub enum PageTreeKids {
    Page(Page),
    PageTreeNode(Rc<PageTreeNode>),
}

impl PageTreeKids {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: Rc<XRef>) -> Self {
        match Object::new(bytes, curr_idx, xref.clone()) {
            Object::Dictionary(dict) => match dict.get("Type") {
                Some(Object::Name(name)) => match name.as_str() {
                    "Pages" => {
                        PageTreeKids::PageTreeNode(PageTreeNode::new(bytes, curr_idx, xref.clone()))
                    }
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

#[derive(Debug, PartialEq, Clone)]
pub struct Font {
    pub subtype: Name,
    name: Option<Name>,
    pub base_font: Name,
    first_char: Option<Number>, // Number -> Integer
    last_char: Option<Number>,  // Number -> Integer
    widths: Option<Vec<Number>>,
    pub to_unicode: Option<ToUnicodeCMap>,
    encoding: Option<Name>,
}

impl Font {
    pub fn estimate_space_width(&self) -> Number {
        match self.get_width(b' ') {
            Ok(space_width) => space_width,
            Err(_) => match self.average_width() {
                Ok(average_width) => average_width,
                Err(_) => Number::Integer(200),
            },
        }
    }

    fn average_width(&self) -> Result<Number, &str> {
        if let Some(widths) = &self.widths {
            let mut sum = Number::Real(0.0);
            for n in widths {
                sum = sum + n.clone();
            }
            Ok(sum / Number::Integer(widths.len() as i32) / Number::Real(1000.0))
        } else {
            Err("Font does not contain widths")
        }
    }

    // horizontal displacement
    pub fn get_width(&self, c: u8) -> Result<Number, &str> {
        if let Some(Number::Integer(first_char)) = &self.first_char {
            if i32::from(c) < *first_char {
                return Err("Cannot get character width from the current font range");
            }
        }
        match &self.widths {
            Some(widths) => {
                let c_offset: usize =
                    usize::from(c) - usize::from(self.first_char.clone().unwrap());
                match widths.get(c_offset) {
                    Some(n) => Ok(n.clone() / Number::Real(1000.0)), // cf note on TJ in page 408
                    _ => Err("Width of char was not found in the font"),
                }
            }
            None => Err("No character widths stored in the current font"),
        }
    }
}

impl Display for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Subtype: {:?}\nName: {:?}\nBaseFont: {:?}\nFirstChar: {:?}\nLastChar: {:?}\nWidths: {:?}\nToUnicode: {:?}", self.subtype, self.name, self.base_font, self.first_char, self.last_char, self.widths, self.to_unicode
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
            widths: match value.get("Widths") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => match Object::new(bytes, address, xref.clone()) {
                            Object::Array(a) => Some(a.iter().map(|o| match o {
                                Object::Numeric(n) => n.clone(),
                                o => panic!("Widths should be an array containing only numbers, found {o:?}")
                            }).collect()),
                            o => panic!("ToUnicode should be a stream object, found {o:?}"),
                        },
                        None => panic!("ToUnicode stream object not found in xref table"),
                    }
                }
                Some(Object::Array(a)) => Some(
                    a.iter()
                        .map(|o| match o {
                            Object::Numeric(n) => n.clone(),
                            o => panic!(
                                "Widths should be an array containing only numbers, found {o:?}"
                            ),
                        })
                        .collect(),
                ),
                Some(o) => panic!("Widths should be an array of objects, found {o:?}"),
                None => None,
            },
            to_unicode: match value.get("ToUnicode") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => match Object::new(bytes, address, xref.clone()) {
                            Object::Stream(stream) => Some(ToUnicodeCMap::from(
                                String::from_utf8_lossy(&Stream::from(stream).get_data())
                                    .to_string(),
                            )),
                            o => panic!("ToUnicode should be a stream object, found {o:?}"),
                        },
                        None => panic!("ToUnicode stream object not found in xref table"),
                    }
                }
                None => None,
                _ => panic!("ToUnicode should be an indirect object"),
            },
            encoding: match value.get("Encoding") {
                Some(Object::Name(name)) => Some(name.clone()),
                Some(_) => None, // dictionnary encoding not supported at the moment
                None => None,
            },
        }
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct FontMap(pub HashMap<Name, Font>);

impl Display for FontMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fonts = self
            .0
            .values()
            .map(|font| format!("{font}\n"))
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{fonts}")
    }
}

impl From<Dictionary<'_>> for FontMap {
    fn from(value: Dictionary) -> Self {
        FontMap(
            value
                .iter()
                .map(|(key, value)| match value {
                    Object::Ref((obj, gen), xref, bytes) => match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => {
                            match Object::new(bytes, address, xref.clone()) {
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

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Resources {
    pub font: Option<FontMap>,
}

impl Resources {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: Rc<XRef>) -> Self {
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
                        FontMap::from(match Object::new(bytes, address, xref.clone()) {
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

#[derive(Debug)]
pub struct PageTreeNode {
    parent: RefCell<Weak<PageTreeNode>>, // PageTreeNode parent
    kids: Vec<PageTreeKids>,             // PageTreeNode kids can be a Page or a PageTreeNode
    // count: Number,                       // Number of leaf nodes
    // Inheritables (cf page 149)
    // rotate: Option<Number>, // Number of degrees by which the page should be rotated clockwise when displayeds
    // crop_box: Option<Rectangle>, // CropBox Rectangle
    // media_box: Option<Rectangle>, // MediaBox Rectangle
    resources: Option<Resources>, // Resource dictionary
}

impl PageTreeNode {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: Rc<XRef>) -> Rc<Self> {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => {
                let page_tree_node = Rc::new(Self::from(dict));
                // update parent weak reference of children
                page_tree_node.kids.iter().for_each(|k| match k {
                    PageTreeKids::Page(p) => {
                        *p.parent.borrow_mut() = Rc::downgrade(&page_tree_node)
                    }
                    PageTreeKids::PageTreeNode(p) => {
                        *p.parent.borrow_mut() = Rc::downgrade(&page_tree_node)
                    }
                });
                page_tree_node
            }
            _ => panic!("Trailer should be a dictionary"),
        }
    }

    fn get_resources(&self) -> Option<Resources> {
        match &self.resources {
            Some(r) => Some(r.clone()), // TODO : improve with smart pointer instead of cloning
            None => match self.parent.borrow().upgrade() {
                Some(p) => p.get_resources(),
                None => None,
            },
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
            parent: RefCell::new(Weak::new()),
            kids: match value.get("Kids").unwrap() {
                Object::Array(arr) => arr
                    .iter()
                    .map(|kid| match kid {
                        Object::Ref((obj, gen), xref, bytes) => {
                            match xref.get_and_fix(&(*obj, *gen), bytes) {
                                Some(address) => PageTreeKids::new(bytes, address, xref.clone()),
                                None => panic!("Kid not found in xref table"),
                            }
                        }
                        _ => panic!("Kid should be an indirect object"),
                    })
                    .collect(),
                _ => panic!("Kids should be an array"),
            },
            // count: match value.get("Count").unwrap() {
            //     Object::Numeric(n) => n.clone(),
            //     _ => panic!("Count should be a numeric"),
            // },
            // rotate: match value.get("Rotate") {
            //     Some(Object::Numeric(n)) => Some(n.clone()),
            //     None => None,
            //     _ => panic!("Rotate should be a numeric"),
            // },
            // crop_box: match value.get("CropBox") {
            //     Some(Object::Array(arr)) => Some(Rectangle::from(arr.clone())),
            //     None => None,
            //     _ => panic!("CropBox should be an array"),
            // },
            // media_box: match value.get("MediaBox") {
            //     Some(Object::Array(arr)) => Some(Rectangle::from(arr.clone())),
            //     None => None,
            //     _ => panic!("MediaBox should be an array"),
            // },
            resources: match value.get("Resources") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Some(Resources::new(bytes, address, xref.clone())),
                        None => panic!("Kid not found in xref table"),
                    }
                }
                None => None,
                _ => panic!("Resources should be an indirect object"),
            },
        }
    }
}

#[derive(Debug)]
pub struct Page {
    parent: RefCell<Weak<PageTreeNode>>, // Page leaf parent
    // last_modified: Option<String>,       // Date and time of last modification
    resources: Option<Resources>, // Resource dictionary (inheritable from PageTreeNode)
    // media_box: Option<Rectangle>,        // MediaBox rectangle (inheritable from PageTreeNode)
    // crop_box: Option<Rectangle>,         // CropBox rectangle (inheritable from PageTreeNode)
    contents: Option<Stream>, // Page content
}

impl Page {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: Rc<XRef>) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }

    // Get resources from Page or parent if missing
    pub fn get_resources(&self) -> Box<Resources> {
        match &self.resources {
            Some(r) => Box::new(r.clone()),
            None => match self.parent.borrow().upgrade() {
                Some(p) => match p.get_resources() {
                    Some(r) => Box::new(r),
                    None => panic!("Resources not found for current Page and in parent tree"),
                },
                None => panic!("Unable to retrieve Page Resource, current page with no parent"),
            },
        }
    }

    pub fn extract(&self, e: Extract) -> String {
        match e {
            Extract::Text => self.extract_text(false),
            Extract::Chars => self.extract_text(true),
            Extract::RawContent => self.extract_stream(),
            Extract::Font => self.extract_font(),
        }
    }

    fn extract_font(&self) -> String {
        match self.get_resources().font {
            Some(font_map) => font_map.to_string(),
            None => panic!("Missing font in current page resources"),
        }
    }

    fn extract_text(&self, char: bool) -> String {
        let content_bytes = self.extract_stream();
        let mut text_content =
            content::TextContent::new(content_bytes.as_bytes(), self.get_resources());
        text_content.get_text(char)
    }

    fn extract_stream(&self) -> String {
        // Extract text
        match &self.contents {
            Some(stream) => String::from_utf8_lossy(&stream.get_data()).to_string(),
            None => panic!("Contents should not be empty"),
        }
    }
}

impl From<Dictionary<'_>> for Page {
    fn from(value: Dictionary) -> Self {
        Page {
            parent: RefCell::new(Weak::new()),
            // last_modified: match value.get("LastModified") {
            //     Some(Object::String(s)) => Some(s.clone()),
            //     None => None,
            //     _ => panic!("LastModified should be a string"),
            // },
            resources: match value.get("Resources").unwrap() {
                Object::Dictionary(t) => Some(Resources::from(t.clone())),
                Object::Ref((obj, gen), xref, bytes) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Some(Resources::new(bytes, address, xref.clone())),
                        None => panic!("Resource dictionnary address not found in xref keys"),
                    }
                }
                t => panic!("Resources should be an dictionary object {t:?}"),
            },
            // media_box: match value.get("MediaBox") {
            //     Some(Object::Array(arr)) => Some(Rectangle::from(arr.clone())),
            //     Some(a) => panic!("MediaBox should be an array; found {a:?}"),
            //     None => None,
            // },
            // crop_box: match value.get("CropBox") {
            //     Some(Object::Array(arr)) => Some(Rectangle::from(arr.clone())),
            //     Some(a) => panic!("CropBox should be an array; found {a:?}"),
            //     None => None,
            // },
            contents: match value.get("Contents") {
                Some(Object::Ref((obj, gen), xref, bytes)) => {
                    match xref.get_and_fix(&(*obj, *gen), bytes) {
                        Some(address) => Some(Stream::new(bytes, address, xref.clone())),
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
#[derive(Debug)]
pub struct Catalog {
    // The page tree node that is the root of the document’s page tree
    // Must be an indirect reference
    pub pages: Option<Rc<PageTreeNode>>,
}

impl Catalog {
    pub fn new(bytes: &[u8], curr_idx: usize, xref: Rc<XRef>) -> Self {
        match Object::new(bytes, curr_idx, xref) {
            Object::Dictionary(dict) => Self::from(dict),
            o => panic!("Catalog should be a dictionary, found {o:?}"),
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
                    .map(|address| PageTreeNode::new(bytes, address, xref.clone())),
                _ => panic!("Pages should be an indirect object"),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::xref::XRefTable;

    use super::*;

    #[test]
    fn test_catalog() {
        let catalog = Catalog::new(b"1 0 obj  % entry point\n    <<\n      /Type /Catalog\n      /Pages 2 0 R\n    >>\n    endobj".as_slice(), 0, Rc::new(XRef::XRefTable(XRefTable::default())));
        assert!(catalog.pages.is_none())
    }
}
