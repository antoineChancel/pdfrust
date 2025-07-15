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
    object::{Array, Dictionary, Lemmatizer, Name, Object, Token},
    xref::XRef,
    Extract,
};

use crate::object::Stream as StreamObject;

#[derive(Debug, PartialEq)]
pub struct Rectangle([Number; 4]);

impl From<Array> for Rectangle {
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

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
pub enum Indirect<T: From<Object>> {
    Ref(i32, i32),
    T(T),
}

impl<T: From<Object> + Clone> Indirect<T> {
    pub fn read(&self, xref: &mut XRef) -> T {
        match self {
            Indirect::Ref(obj, gen) => T::from(xref.get_and_fix_object(&(*obj, *gen))),
            Indirect::T(obj) => obj.clone(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct ObjectStreams {
    length: Indirect<Number>,
    filter: Option<Filter>,
    n: usize,     // number of compressed objects in the stream
    first: usize, // byte offset of first compressed object
    data: Vec<u8>,
    table: HashMap<usize, usize>, // object_number -> byte_offset
}

impl ObjectStreams {
    fn from(value: StreamObject) -> Self {
        let filter = match value.header.get("Filter") {
            Some(Object::Name(name)) => Some(Filter::from(name.clone())),
            None => None,
            _ => panic!("Filter should be a name"),
        };
        let data = Self::deflate(&value.bytes, &filter);
        ObjectStreams {
            length: match value.header.get("Length").unwrap() {
                Object::Numeric(n) => Indirect::T(n.clone()),
                Object::Ref((obj, gen)) => Indirect::Ref(*obj, *gen),
                _ => panic!("Length should be a numeric"),
            },
            filter,
            n: match value.header.get("N") {
                Some(Object::Numeric(Number::Integer(n))) => *n as usize,
                Some(_) => panic!("N should be an integer"),
                None => panic!("N is mandatory in an object streams"),
            },
            first: match value.header.get("First") {
                Some(Object::Numeric(Number::Integer(first))) => *first as usize,
                Some(_) => panic!("First should be an integer"),
                None => panic!("First is mandatory in an object streams"),
            },
            data,
            table: HashMap::new(),
        }
    }
}

impl ObjectStreams {
    fn create_table(&mut self) {
        let mut parser = Lemmatizer::new(self.data.as_slice());
        self.table.clear();
        loop {
            let obj_num = match parser.next() {
                Some(Token::Numeric(Number::Integer(n))) => n as usize,
                Some(_) => break,
                None => panic!("Unable to read object streams object number"),
            };
            let offset = match parser.next() {
                Some(Token::Numeric(Number::Integer(offset))) => offset as usize,
                Some(_) => break,
                None => panic!("Unable to read object streams offset"),
            };
            self.table.insert(obj_num, offset);
        }
    }

    fn get(&self, obj_num: usize) -> Object {
        Object::new(&self.data[self.table.get(&obj_num).unwrap() + self.first..])
    }
}

impl Deflate for ObjectStreams {}

#[derive(Debug, PartialEq, Clone)]
struct Stream {
    length: Indirect<Number>,
    filter: Option<Filter>,
    data: Vec<u8>,
}

impl From<Object> for Stream {
    fn from(value: Object) -> Self {
        if let Object::Stream(s) = value {
            Self::from(s)
        } else {
            panic!("")
        }
    }
}

impl Stream {
    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

trait Deflate {
    fn deflate(bytes: &[u8], filter: &Option<Filter>) -> Vec<u8> {
        match filter {
            Some(Filter::FlateDecode) => flate_decode(bytes),
            // Some(f) => panic!("Filter {f:?} is not supported at the moment"),
            None => bytes.to_vec(), // if no filter in header, keep data as is
        }
    }
}

impl Deflate for Stream {}

impl Stream {
    fn from(object: StreamObject) -> Self {
        let filter = match object.header.get("Filter") {
            Some(Object::Name(name)) => Some(Filter::from(name.clone())),
            None => None,
            _ => panic!("Filter should be a name"),
        };
        let data = Self::deflate(&object.bytes, &filter);
        Stream {
            length: match object.header.get("Length").unwrap() {
                Object::Numeric(n) => Indirect::T(n.clone()),
                Object::Ref((obj, gen)) => Indirect::Ref(*obj, *gen),
                _ => panic!("Length should be a numeric"),
            },
            filter,
            data,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PageTreeKids {
    Page(Indirect<Page>),
    PageTreeNode(Indirect<PageTreeNode>),
}

impl PageTreeKids {
    pub fn new(bytes: &[u8], curr_idx: usize) -> Self {
        match Object::new(&bytes[curr_idx..]) {
            Object::Dictionary(dict) => match dict.get("Type") {
                Some(Object::Name(name)) => match name.as_str() {
                    "Pages" => PageTreeKids::PageTreeNode(Indirect::T(PageTreeNode::new(
                        &bytes[curr_idx..],
                    ))),
                    "Page" => PageTreeKids::Page(Indirect::T(Page::new(&bytes[curr_idx..]))),
                    _ => panic!("Unexpected dictionnary type"),
                },
                Some(o) => panic!("Type should be a name, found object {o:?}"),
                None => panic!("Type was not found in dictionnary, {dict:?}"),
            },
            _ => panic!("PageTreeKids should be a dictionary"),
        }
    }

    pub fn extract(&self, e: Extract, xref: &mut XRef) -> String {
        match self {
            PageTreeKids::Page(page) => page.read(xref).extract(e, xref),
            PageTreeKids::PageTreeNode(page_tree_node) => {
                page_tree_node.read(xref).extract(e, xref)
            }
        }
    }
}

impl From<Object> for PageTreeKids {
    fn from(value: Object) -> Self {
        match value {
            Object::Dictionary(ref dict) => match dict.get("Type") {
                Some(Object::Name(name)) => match name.as_str() {
                    "Pages" => PageTreeKids::PageTreeNode(Indirect::T(PageTreeNode::from(value))),
                    "Page" => PageTreeKids::Page(Indirect::T(Page::from(value))),
                    _ => panic!("Unexpected dictionnary type"),
                },
                Some(o) => panic!("Type should be a name, found object {o:?}"),
                None => panic!("Type was not found in dictionnary, {dict:?}"),
            },
            _ => panic!("PageTreeKids should be a dictionary"),
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
    widths: Option<Indirect<Vec<Number>>>,
    pub to_unicode: Option<Indirect<ToUnicodeCMap>>,
    encoding: Option<Name>,
}

impl Font {
    pub fn estimate_space_width(&self, xref: &mut XRef) -> Number {
        match self.get_width(b' ', xref) {
            Ok(space_width) => space_width,
            Err(_) => match self.average_width(xref) {
                Ok(average_width) => average_width,
                Err(_) => Number::Integer(200),
            },
        }
    }

    fn average_width(&self, xref: &mut XRef) -> Result<Number, &str> {
        // Font contains a width
        if let Some(widths) = &self.widths {
            let mut sum = Number::Real(0.0);
            let widths = &widths.read(xref);
            for n in widths {
                sum = sum + n.clone();
            }
            Ok(sum / Number::Integer(widths.len() as i32) / Number::Real(1000.0))
        } else {
            Err("Font does not contain widths")
        }
    }

    // horizontal displacement
    pub fn get_width(&self, c: u8, xref: &mut XRef) -> Result<Number, &str> {
        if let Some(Number::Integer(first_char)) = &self.first_char {
            if i32::from(c) < *first_char {
                return Err("Cannot get character width from the current font range");
            }
        }
        match &self.widths {
            Some(widths) => {
                let c_offset: usize =
                    usize::from(c) - usize::from(self.first_char.clone().unwrap());
                let widths = widths.read(xref);
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

impl From<Object> for Font {
    fn from(value: Object) -> Self {
        if let Object::Dictionary(d) = value {
            Self::from(d)
        } else {
            panic!("Unable to create Font from object")
        }
    }
}

impl From<Dictionary> for Font {
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
                Some(Object::Ref((obj, gen))) => Some(Indirect::Ref(*obj, *gen)),
                Some(Object::Array(a)) => Some(Indirect::T(
                    a.iter()
                        .map(|o| match o {
                            Object::Numeric(n) => n.clone(),
                            o => panic!(
                                "Widths should be an array containing only numbers, found {o:?}"
                            ),
                        })
                        .collect(),
                )),
                Some(o) => panic!("Widths should be an array of objects, found {o:?}"),
                None => None,
            },
            to_unicode: match value.get("ToUnicode") {
                Some(Object::Ref((obj, gen))) => Some(Indirect::Ref(*obj, *gen)),
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
pub struct FontMap(pub HashMap<Name, Indirect<Font>>);

impl Display for FontMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fonts = self
            .0
            .values()
            .map(|font| format!("{font:?}\n"))
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{fonts}")
    }
}

impl FontMap {
    fn from(value: Dictionary) -> Self {
        FontMap(
            value
                .iter()
                .map(|(key, value)| match value {
                    Object::Ref((obj, gen)) => (key.clone(), Indirect::Ref(*obj, *gen)),
                    _ => panic!("FontMap values should be indirect objects"),
                })
                .collect(),
        )
    }
}

impl From<Object> for FontMap {
    fn from(value: Object) -> Self {
        if let Object::Dictionary(d) = value {
            Self::from(d)
        } else {
            panic!("Unable to create FontMap from object")
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Resources {
    pub font: Option<Indirect<FontMap>>,
}

impl From<Object> for Resources {
    fn from(value: Object) -> Self {
        if let Object::Dictionary(d) = value {
            Self::from(d)
        } else {
            panic!("Unable to crate Resources from object {value:?}")
        }
    }
}

impl Resources {
    pub fn new(bytes: &[u8]) -> Self {
        match Object::new(bytes) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }
}

impl From<Dictionary> for Resources {
    fn from(value: Dictionary) -> Self {
        Resources {
            font: match value.get("Font") {
                Some(Object::Ref((obj, gen))) => Some(Indirect::Ref(*obj, *gen)),
                Some(Object::Dictionary(t)) => Some(Indirect::T(FontMap::from(t.clone()))),
                None => None,
                f => panic!("Font should be an indirect object or a dictionary; found {f:?}"),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageTreeNode {
    parent: Option<Box<Indirect<PageTreeNode>>>, // PageTreeNode parent
    kids: Vec<Indirect<PageTreeKids>>, // PageTreeNode kids can be a Page or a PageTreeNode
    resources: Option<Indirect<Resources>>, // Resource dictionary
}

impl From<Object> for PageTreeNode {
    fn from(value: Object) -> Self {
        if let Object::Dictionary(dict) = value {
            Self::from(dict)
        } else {
            panic!("Trailer should be a dictionary");
        }
    }
}

impl PageTreeNode {
    pub fn new(bytes: &[u8]) -> Self {
        PageTreeNode::from(Object::new(bytes))
    }

    fn get_resources(&self, xref: &mut XRef) -> Option<Resources> {
        match &self.resources {
            Some(r) => Some(r.read(xref)), // TODO : improve with smart pointer instead of cloning
            None => {
                let p = &self.parent;
                p.clone().unwrap().read(xref).get_resources(xref)
            }
        }
    }

    pub fn extract(&self, e: Extract, xref: &mut XRef) -> String {
        self.kids
            .iter()
            .map(|kid| kid.read(xref).extract(e.clone(), xref))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl From<Dictionary> for PageTreeNode {
    fn from(value: Dictionary) -> Self {
        PageTreeNode {
            parent: match value.get("Parent") {
                Some(Object::Ref((obj, gen))) => Some(Box::new(Indirect::Ref(*obj, *gen))),
                None => None, // only valid for root node
                _ => panic!("PageTreeNode Parent should be an indirect object"),
            },
            kids: match value.get("Kids") {
                Some(Object::Array(arr)) => arr
                    .iter()
                    .map(|kid| match kid {
                        Object::Ref((obj, gen)) => {
                            Indirect::Ref(*obj, *gen)
                        }
                        _ => panic!("Kid should be an indirect object"),
                    })
                    .collect(),
                None => panic!("PageTreeNode must have Kids"),
                _ => panic!("Kids should be an array"),
            },
            resources: match value.get("Resources") {
                Some(Object::Ref((obj, gen))) => Some(Indirect::Ref(*obj, *gen)),
                None => None,
                _ => panic!("Resources should be an indirect object"),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Page {
    parent: RefCell<Weak<PageTreeNode>>, // Page leaf parent
    // last_modified: Option<String>,       // Date and time of last modification
    resources: Option<Indirect<Resources>>, // Resource dictionary (inheritable from PageTreeNode)
    // media_box: Option<Rectangle>,        // MediaBox rectangle (inheritable from PageTreeNode)
    // crop_box: Option<Rectangle>,         // CropBox rectangle (inheritable from PageTreeNode)
    contents: Option<Indirect<Stream>>, // Page content
}

impl From<Object> for Page {
    fn from(value: Object) -> Self {
        if let Object::Dictionary(d) = value {
            Self::from(d)
        } else {
            panic!()
        }
    }
}

impl Page {
    pub fn new(bytes: &[u8]) -> Self {
        match Object::new(bytes) {
            Object::Dictionary(dict) => Self::from(dict),
            _ => panic!("Trailer should be a dictionary"),
        }
    }

    // Get resources from Page or parent if missing
    pub fn get_resources(&self, xref: &mut XRef) -> Box<Resources> {
        match &self.resources {
            Some(r) => Box::new(r.read(xref).clone()),
            None => match self.parent.borrow().upgrade() {
                Some(p) => match p.get_resources(xref) {
                    Some(r) => Box::new(r),
                    None => panic!("Resources not found for current Page and in parent tree"),
                },
                None => panic!("Unable to retrieve Page Resource, current page with no parent"),
            },
        }
    }

    pub fn extract(&self, e: Extract, xref: &mut XRef) -> String {
        match e {
            Extract::Text => self.extract_text(false, xref),
            Extract::Chars => self.extract_text(true, xref),
            Extract::RawContent => self.extract_stream(xref),
            Extract::Font => self.extract_font(xref),
            Extract::Xref => String::new(),
        }
    }

    fn extract_font(&self, xref: &mut XRef) -> String {
        match self.get_resources(xref).font {
            Some(font_map) => font_map.read(xref).to_string(),
            None => panic!("Missing font in current page resources"),
        }
    }

    fn extract_text(&self, char: bool, xref: &mut XRef) -> String {
        let content_bytes = self.extract_stream(xref);
        let mut text_content =
            content::TextContent::new(content_bytes.as_bytes(), self.get_resources(xref));
        text_content.get_text(char, xref)
    }

    fn extract_stream(&self, xref: &mut XRef) -> String {
        // Extract text
        match &self.contents {
            Some(stream) => String::from_utf8_lossy(&stream.read(xref).get_data()).to_string(),
            None => panic!("Contents should not be empty"),
        }
    }
}

impl From<Dictionary> for Page {
    fn from(value: Dictionary) -> Self {
        Page {
            parent: RefCell::new(Weak::new()),
            resources: match value.get("Resources").unwrap() {
                Object::Dictionary(t) => Some(Indirect::T(Resources::from(t.clone()))),
                Object::Ref((obj, gen)) => Some(Indirect::Ref(*obj, *gen)),
                t => panic!("Resources should be an dictionary object {t:?}"),
            },
            contents: match value.get("Contents") {
                Some(Object::Ref((obj, gen))) => Some(Indirect::Ref(*obj, *gen)),
                None => None,
                Some(Object::Array(references)) => {
                    if references.len() == 1 {
                        match references.first() {
                            Some(Object::Ref((obj, gen))) => Some(Indirect::Ref(*obj, *gen)),
                            _ => panic!("Unreadable array of content streams"),
                        }
                    } else {
                        panic!("Page content with array of streams is not supported")
                    }
                }
                Some(o) => panic!("Contents should be an indirect object, found {:?}", o),
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
    pub pages: Option<Rc<Indirect<PageTreeNode>>>,
}

impl Catalog {
    pub fn new(bytes: &[u8], curr_idx: usize) -> Self {
        match Object::new(&bytes[curr_idx..]) {
            Object::Dictionary(dict) => Self::from(dict),
            Object::Stream(stream) => {
                let mut stream = ObjectStreams::from(stream);
                stream.create_table();
                let obj = stream.get(2);
                panic!("{obj:?}");
            }
            o => panic!("Catalog should be a dictionary, found {o:?}"),
        }
    }

    pub fn extract(&self, e: Extract, xref: &mut XRef) -> String {
        match &self.pages {
            Some(page_tree_node) => page_tree_node.read(xref).extract(e, xref),
            None => panic!("Pages should not be empty"),
        }
    }

    fn from(value: Dictionary) -> Self {
        Catalog {
            pages: match value.get("Pages").unwrap() {
                Object::Ref((obj, gen)) => Some(Indirect::Ref(*obj, *gen).into()),
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
        let catalog = Catalog::new(b"1 0 obj  % entry point\n    <<\n      /Type /Catalog\n      /Pages 2 0 R\n    >>\n    endobj".as_slice(), 0);
        assert!(catalog.pages.is_none())
    }
}
