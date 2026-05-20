/// PDF/UA (Universal Accessibility) support via Tagged PDF.
///
/// Adds a structure tree to the PDF that maps visual content to logical
/// document structure (headings, paragraphs, lists, tables, images).

/// Post-process PDF bytes to add Tagged PDF structure.
pub fn make_tagged_pdf(pdf_bytes: &[u8], structure: &[DocStructureNode]) -> Vec<u8> {
    let Ok(mut doc) = lopdf::Document::load_mem(pdf_bytes) else {
        return pdf_bytes.to_vec();
    };

    // Create StructTreeRoot
    let mut struct_kids = Vec::new();

    for node in structure {
        let elem_ref = add_struct_elem(&mut doc, node);
        struct_kids.push(lopdf::Object::Reference(elem_ref));
    }

    let struct_tree = lopdf::Dictionary::from_iter(vec![
        ("Type", lopdf::Object::Name(b"StructTreeRoot".to_vec())),
        ("K", lopdf::Object::Array(struct_kids)),
    ]);
    let struct_tree_ref = doc.add_object(lopdf::Object::Dictionary(struct_tree));

    // Add StructTreeRoot to catalog
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.set("StructTreeRoot", lopdf::Object::Reference(struct_tree_ref));
        catalog.set("MarkInfo", lopdf::Object::Dictionary(
            lopdf::Dictionary::from_iter(vec![
                ("Marked", lopdf::Object::Boolean(true)),
            ]),
        ));
        // Set document language
        catalog.set("Lang", lopdf::Object::String(b"en".to_vec(), lopdf::StringFormat::Literal));
    }

    let mut output = Vec::new();
    let _ = doc.save_to(&mut output);
    output
}

fn add_struct_elem(doc: &mut lopdf::Document, node: &DocStructureNode) -> lopdf::ObjectId {
    let mut kids = Vec::new();
    for child in &node.children {
        let child_ref = add_struct_elem(doc, child);
        kids.push(lopdf::Object::Reference(child_ref));
    }

    let mut dict = lopdf::Dictionary::from_iter(vec![
        ("Type", lopdf::Object::Name(b"StructElem".to_vec())),
        ("S", lopdf::Object::Name(node.role.pdf_name().into())),
    ]);

    if !kids.is_empty() {
        dict.set("K", lopdf::Object::Array(kids));
    }

    if let Some(alt) = &node.alt_text {
        dict.set("Alt", lopdf::Object::String(alt.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    }

    if let Some(lang) = &node.lang {
        dict.set("Lang", lopdf::Object::String(lang.as_bytes().to_vec(), lopdf::StringFormat::Literal));
    }

    doc.add_object(lopdf::Object::Dictionary(dict))
}

/// Logical role of a structure element.
#[derive(Debug, Clone)]
pub enum StructureRole {
    Document,
    Part,
    Heading(u8), // H1-H6
    Paragraph,
    List,
    ListItem,
    Table,
    TableRow,
    TableHeader,
    TableData,
    Figure,
    BlockQuote,
    Code,
    Span,
}

impl StructureRole {
    fn pdf_name(&self) -> &[u8] {
        match self {
            StructureRole::Document => b"Document",
            StructureRole::Part => b"Part",
            StructureRole::Heading(1) => b"H1",
            StructureRole::Heading(2) => b"H2",
            StructureRole::Heading(3) => b"H3",
            StructureRole::Heading(4) => b"H4",
            StructureRole::Heading(5) => b"H5",
            StructureRole::Heading(_) => b"H6",
            StructureRole::Paragraph => b"P",
            StructureRole::List => b"L",
            StructureRole::ListItem => b"LI",
            StructureRole::Table => b"Table",
            StructureRole::TableRow => b"TR",
            StructureRole::TableHeader => b"TH",
            StructureRole::TableData => b"TD",
            StructureRole::Figure => b"Figure",
            StructureRole::BlockQuote => b"BlockQuote",
            StructureRole::Code => b"Code",
            StructureRole::Span => b"Span",
        }
    }
}

/// A node in the document structure tree.
#[derive(Debug, Clone)]
pub struct DocStructureNode {
    pub role: StructureRole,
    pub alt_text: Option<String>,
    pub lang: Option<String>,
    pub children: Vec<DocStructureNode>,
}

/// Build document structure from a styled tree.
pub fn build_structure(tree: &crate::style::StyledNode) -> Vec<DocStructureNode> {
    let mut nodes = Vec::new();
    build_structure_recursive(tree, &mut nodes);
    nodes
}

fn build_structure_recursive(node: &crate::style::StyledNode, out: &mut Vec<DocStructureNode>) {
    let role = match node.tag.as_str() {
        "h1" => Some(StructureRole::Heading(1)),
        "h2" => Some(StructureRole::Heading(2)),
        "h3" => Some(StructureRole::Heading(3)),
        "h4" => Some(StructureRole::Heading(4)),
        "h5" => Some(StructureRole::Heading(5)),
        "h6" => Some(StructureRole::Heading(6)),
        "p" => Some(StructureRole::Paragraph),
        "ul" | "ol" => Some(StructureRole::List),
        "li" => Some(StructureRole::ListItem),
        "table" => Some(StructureRole::Table),
        "tr" => Some(StructureRole::TableRow),
        "th" => Some(StructureRole::TableHeader),
        "td" => Some(StructureRole::TableData),
        "blockquote" => Some(StructureRole::BlockQuote),
        "pre" | "code" => Some(StructureRole::Code),
        "img" => {
            let alt = node.attrs.iter()
                .find(|(k, _)| k == "alt")
                .map(|(_, v)| v.clone());
            out.push(DocStructureNode {
                role: StructureRole::Figure,
                alt_text: alt,
                lang: None,
                children: Vec::new(),
            });
            return;
        }
        "figure" => Some(StructureRole::Figure),
        _ => None,
    };

    if let Some(role) = role {
        let mut children = Vec::new();
        for child in &node.children {
            if let crate::style::StyledContent::Element(child_node) = child {
                build_structure_recursive(child_node, &mut children);
            }
        }
        out.push(DocStructureNode {
            role,
            alt_text: None,
            lang: None,
            children,
        });
    } else {
        // Container: recurse
        for child in &node.children {
            if let crate::style::StyledContent::Element(child_node) = child {
                build_structure_recursive(child_node, out);
            }
        }
    }
}
