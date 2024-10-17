// type Rectangle = [Numeric; 4];
// type Stream<'a> = &'a [u8];

// #[derive(Debug)]
// pub enum PageTreeKids {
//     Page(Page),
//     PageTreeNode(PageTreeNode),
// }

// impl PageTreeKids {
//     fn new(bytes: &[u8], xref: &XrefTable) -> PageTreeKids {
//         // Read header of dictionary
//         let mut pdf = Tokenizer::new(bytes);

//         println!("PageTreeKids bytes: {:?}", std::str::from_utf8(bytes));

//         // Consume object header
//         match pdf.next() {
//             Some(Token::IndirectRef(_, _)) => (),
//             Some(t) => panic!("PageTreeKids should start with an indirect reference;  found {t:?}"),
//             None => panic!("PageTreeKids should start with an indirect reference"),
//         };

//         match pdf.next() {
//             Some(Token::DictBegin) => (), // Ok continue
//             Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
//             None => panic!("PageTreeNodeRoot should be a dictionnary"),
//         };

//         // check type of kid
//         while let Some(t) = pdf.next() {
//             match t {
//                 Token::Name("Type") => match pdf.next() {
//                     Some(Token::Name("Pages")) => {
//                         return PageTreeKids::PageTreeNode(PageTreeNode::new(&bytes, &xref))
//                     }
//                     Some(Token::Name("Page")) => {
//                         return PageTreeKids::Page(Page::new(&bytes, &xref))
//                     }
//                     Some(t) => panic!("Unexpected dictionnary type; found token {t:?}"),
//                     None => panic!("Unexpected dictionnary type"),
//                 },
//                 Token::DictEnd => break,
//                 a => panic!("Unexpected key was found in dictionnary catalog {a:?}"),
//             }
//         }
//         panic!("PageTreeKid should have a Type key");
//     }
// }

// enum PageTreeParent {
//     PageTreeNodeRoot(PageTreeNodeRoot),
//     PageTreeNode(PageTreeNode),
// }

// #[derive(Debug)]
// pub struct PageTreeNodeRoot {
//     kids: Vec<PageTreeKids>, // PageTreeNode kids can be a Page or a PageTreeNode
//     count: Numeric,          // Number of leaf nodes
//     // Inheritables (cf page 149)
//     rotate: Option<Numeric>, // Number of degrees by which the page should be rotated clockwise when displayeds
//     crop_box: Option<Rectangle>, // Rectangle
//     media_box: Option<Rectangle>, // Rectangle
//     resources: Option<IndirectObject>, // Resource dictionary
// }

// impl PageTreeNodeRoot {
//     pub fn new(bytes: &[u8], xref: &XrefTable) -> Self {
//         let mut pdf = Tokenizer::new(bytes);

//         println!("PageTreeNodeRoot bytes: {:?}", std::str::from_utf8(bytes));

//         // Consume object header
//         match pdf.next() {
//             Some(Token::IndirectRef(_, _)) => (),
//             Some(t) => {
//                 panic!("PageTreeNodeRoot should start with an indirect reference;  found {t:?}")
//             }
//             None => panic!("PageTreeNodeRoot should start with an indirect reference"),
//         };

//         match pdf.next() {
//             Some(Token::DictBegin) => (), // Ok continue
//             Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
//             None => panic!("PageTreeNodeRoot should be a dictionnary"),
//         };

//         let mut kids = Vec::new();
//         let mut count = 0;
//         let mut rotate = None;
//         let mut crop_box = None;
//         let mut media_box = None;
//         let mut resources = None;

//         while let Some(t) = pdf.next() {
//             match t {
//                 // check if the PageTreeNodeRoot dictionnary is of type Pages
//                 Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Pages"))),
//                 // array of indirect references to the immediate children of this node
//                 Token::Name("Kids") => {
//                     assert_eq!(pdf.next(), Some(Token::ArrayBegin));
//                     while let Ok(indirect_ref) = IndirectObject::try_from(&mut pdf) {
//                         let kids_idx = xref.get(&indirect_ref).unwrap();
//                         kids.push(PageTreeKids::new(&bytes[*kids_idx..], &xref));
//                     }
//                 }
//                 Token::Name("Count") => {
//                     count = match pdf.next() {
//                         Some(Token::Numeric(n)) => Numeric(n),
//                         Some(t) => panic!("Count should be a numeric; found {t:?}"),
//                         None => panic!("Count should be a numeric"),
//                     };
//                 }
//                 Token::Name("Rotate") => {
//                     rotate = match pdf.next() {
//                         Some(Token::Numeric(n)) => Some(Numeric(n)),
//                         Some(t) => panic!("Rotate should be a numeric; found {t:?}"),
//                         None => panic!("Rotate should be a numeric"),
//                     };
//                 }
//                 Token::Name("CropBox") => {
//                     assert_eq!(pdf.next(), Some(Token::ArrayBegin));
//                     let mut crop_box_buff = [Numeric(0); 4];
//                     for i in 0..4 {
//                         crop_box_buff[i] = match pdf.next() {
//                             Some(Token::Numeric(n)) => Numeric(n),
//                             Some(t) => panic!("CropBox should be a numeric; found {t:?}"),
//                             None => panic!("CropBox should be a numeric"),
//                         }
//                     }
//                     assert_eq!(pdf.next(), Some(Token::ArrayEnd));
//                     crop_box = Some(crop_box_buff);
//                 }
//                 Token::Name("MediaBox") => {
//                     assert_eq!(pdf.next(), Some(Token::ArrayBegin));
//                     let mut media_box_buff = [Numeric(0); 4];
//                     for i in 0..4 {
//                         media_box_buff[i] = match pdf.next() {
//                             Some(Token::Numeric(n)) => Numeric(n),
//                             Some(t) => panic!("MediaBox should be a numeric; found {t:?}"),
//                             None => panic!("MediaBox should be a numeric"),
//                         }
//                     }
//                     assert_eq!(pdf.next(), Some(Token::ArrayEnd));
//                     media_box = Some(media_box_buff);
//                 }
//                 Token::Name("Resources") => {
//                     resources = Some(IndirectObject::try_from(&mut pdf).unwrap());
//                 }
//                 Token::DictEnd => break,
//                 a => panic!("Unexpected key was found in dictionnary page tree root node {a:?}"),
//             };
//         }
//         PageTreeNodeRoot {
//             kids,
//             count,
//             rotate,
//             crop_box,
//             media_box,
//             resources,
//         }
//     }
// }

// #[derive(Debug)]
// pub struct PageTreeNode {
//     // parent: PageTreeParent<'a>, // The page tree node's parent
//     kids: Vec<PageTreeKids>, // PageTreeNode kids can be a Page or a PageTreeNode
//     count: Numeric,          // Number of leaf nodes
// }

// impl PageTreeNode {
//     fn new(bytes: &[u8], xref: &XrefTable) -> Self {
//         let mut pdf = Tokenizer::new(bytes);

//         // Consume object header
//         match pdf.next() {
//             Some(Token::IndirectRef(_, _)) => (),
//             Some(t) => {
//                 panic!("PageTreeNodeRoot should start with an indirect reference;  found {t:?}")
//             }
//             None => panic!("PageTreeNodeRoot should start with an indirect reference"),
//         };

//         match pdf.next() {
//             Some(Token::DictBegin) => (), // Ok continue
//             Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
//             None => panic!("PageTreeNodeRoot should be a dictionnary"),
//         };

//         let mut kids = Vec::new();
//         let mut count = 0;

//         while let Some(t) = pdf.next() {
//             match t {
//                 // check if the PageTreeNodeRoot dictionnary is of type Pages
//                 Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Pages"))),
//                 // array of indirect references to the immediate children of this node
//                 Token::Name("Kids") => {
//                     assert_eq!(pdf.next(), Some(Token::ArrayBegin));
//                     while let Ok(indirect_ref) = IndirectObject::try_from(&mut pdf) {
//                         let kids_idx = xref.get(&indirect_ref).unwrap();
//                         kids.push(PageTreeKids::new(&bytes[*kids_idx..], &xref));
//                     }
//                 }
//                 Token::Name("Count") => {
//                     count = match pdf.next() {
//                         Some(Token::Numeric(n)) => Numeric(n),
//                         Some(t) => panic!("Count should be a numeric; found {t:?}"),
//                         None => panic!("Count should be a numeric"),
//                     };
//                 }
//                 Token::DictEnd => break,
//                 a => panic!("Unexpected key was found in dictionnary catalog {a:?}"),
//             };
//         }
//         PageTreeNode { kids, count }
//     }
// }

// #[derive(Debug)]
// struct Page {
//     // parent: PageTreeParent<'a>, // The page tree node's parent
//     last_modified: Option<String>, // Date and time of last modification
//     resources: IndirectObject,     // Resource dictionary
//     media_box: Rectangle,          //rectangle
//                                    // crop_box: Option<Rectangle>,   //rectangle
//                                    // bleed_box: Option<Rectangle>,  //rectangle
//                                    // trim_box: Option<Rectangle>,   //rectangle
//                                    // art_box: Option<Rectangle>,    //rectangle
//                                    // box_color_info: Option<IndirectObject>, // Box color information dictionary
//                                    // contents: Option<Stream<'a>>,  // Content stream; if None Page is empty
//                                    // rotate: Option<Numeric>,
//                                    // group: Option<IndirectObject>, // Group attributes dictionary
//                                    // thumb: Option<Stream<'a>>,
//                                    // b: Option<Vec<IndirectObject>>, // array of indirect references to article beads
//                                    // dur: Option<Numeric>,           // page's display duration
//                                    // trans: Option<IndirectObject>,  // transition dictionary
//                                    // annots: Option<Vec<IndirectObject>>, // array of annotation dictionaries
//                                    // aa: Option<IndirectObject>,     // additional actions dictionary
//                                    // metadata: Option<Stream<'a>>,   // metadata stream of the page
//                                    // piece_info: Option<IndirectObject>, // piece information dictionary
//                                    // struct_parents: Option<Numeric>, // integer
//                                    // id: Option<String>,             // byte string
//                                    // pz: Option<Numeric>,            // integer
//                                    // separation_info: Option<IndirectObject>, // separation information dictionary
//                                    // tabs: Option<Name>, // name specifying the tab order to be used for annotations on the page
//                                    // template_instantiated: Option<Name>, // template dictionary
//                                    // pres_steps: Option<IndirectObject>, // navigation node dictionary
//                                    // user_unit: Option<Numeric>, // number specifying the size of default user space units
//                                    // vp: Option<IndirectObject>, // array of numbers specifying the page's viewport
// }

// impl Page {
//     fn new(bytes: &[u8], xref: &XrefTable) -> Self {
//         let mut pdf = Tokenizer::new(bytes);

//         // Consume object header
//         match pdf.next() {
//             Some(Token::IndirectRef(_, _)) => (),
//             Some(t) => {
//                 panic!("PageTreeNodeRoot should start with an indirect reference;  found {t:?}")
//             }
//             None => panic!("PageTreeNodeRoot should start with an indirect reference"),
//         };

//         // Consume <<
//         match pdf.next() {
//             Some(Token::DictBegin) => (), // Ok continue
//             Some(t) => panic!("PageTreeNodeRoot should be a dictionnary; found {t:?}"),
//             None => panic!("PageTreeNodeRoot should be a dictionnary"),
//         };

//         let mut last_modified = None;
//         let mut resources = None;
//         let mut media_box = None;

//         while let Some(t) = pdf.next() {
//             match t {
//                 // Check if the Page dictionnary is of type Page
//                 Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Page"))),

//                 // Last modified date of the page
//                 Token::Name("LastModified") => match pdf.next() {
//                     Some(Token::LitteralString(s)) => {
//                         last_modified = Some(String::from(std::str::from_utf8(s).unwrap()));
//                     }
//                     Some(t) => panic!("LastModified should be a string; found {t:?}"),
//                     None => panic!("LastModified should be a string"),
//                 },

//                 // Resource dictionnary
//                 Token::Name("Resources") => {
//                     resources = Some(IndirectObject::try_from(&mut pdf).unwrap());
//                 }

//                 // Media box reactangle
//                 Token::Name("MediaBox") => {
//                     assert_eq!(pdf.next(), Some(Token::ArrayBegin));
//                     let mut media_box_buff = [0; 4];
//                     for i in 0..4 {
//                         media_box_buff[i] = match pdf.next() {
//                             Some(Token::Numeric(n)) => n,
//                             Some(t) => panic!("MediaBox should be a numeric; found {t:?}"),
//                             None => panic!("MediaBox should be a numeric"),
//                         }
//                     }
//                     assert_eq!(pdf.next(), Some(Token::ArrayEnd));
//                     media_box = Some(media_box_buff);
//                 }
//                 Token::DictEnd => break,
//                 _ => (),
//             };
//         }
//         Page {
//             last_modified,
//             resources: resources.unwrap(),
//             media_box: media_box.unwrap(),
//         }
//     }
// }

// #[derive(Debug, PartialEq)]
// // Defined in page 139;  commented is to be implemented
// pub struct Catalog {
//     pub pages: Option<IndirectObject>, // The page tree node that is the root of the document’s page tree
// }

// impl From<&[u8]> for Catalog {
//     fn from(bytes: &[u8]) -> Self {
//         let mut pdf = Tokenizer::new(bytes);

//         // Consume object header
//         match pdf.next() {
//             Some(Token::IndirectRef(_, _)) => (),
//             Some(t) => panic!("Catalog should start with an indirect reference;  found {t:?}"),
//             None => panic!("Catalog should start with an indirect reference"),
//         };

//         match pdf.next() {
//             Some(Token::DictBegin) => (),
//             Some(t) => panic!("Catalog should be a dictionnary; found {t:?}"),
//             None => panic!("Catalog should be a dictionnary"),
//         };

//         let mut pages = None;

//         while let Some(t) = pdf.next() {
//             match t {
//                 Token::Name("Type") => assert_eq!(pdf.next(), Some(Token::Name("Catalog"))),
//                 Token::Name("Pages") => match pdf.next() {
//                     Some(Token::IndirectRef(obj, gen)) => pages = Some((obj, gen)),
//                     Some(t) => panic!("Pages should be an indirect reference; found {t:?}"),
//                     None => panic!("Pages should be an indirect reference"),
//                 },
//                 Token::DictEnd => break,
//                 a => panic!("Unexpected key was found in dictionnary catalog {a:?}"),
//             };
//         }
//         Catalog { pages }
//     }
// }

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn test_catalog() {
//         let catalog = Catalog::from(b"1 0 obj  % entry point\n    <<\n      /Type /Catalog\n      /Pages 2 0 R\n    >>\n    endobj".as_slice());
//         assert_eq!(
//             catalog,
//             Catalog {
//                 pages: Some((2, 0))
//             }
//         )
//     }
// }
