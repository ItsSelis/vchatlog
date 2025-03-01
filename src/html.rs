use kuchiki::traits::*;
use kuchiki::NodeRef;

pub struct ParsedData {
    pub classes: Vec<String>,
    pub text: String,
}

/// Parses a given html string and returns a vector of classes used (different styles/spans), and the plain text.
pub fn parse_html(html: &str) -> ParsedData {
    let document = kuchiki::parse_html().one(html);
    let mut parsed_data = ParsedData {
        classes: Vec::new(),
        text: String::new(),
    };

    walk(&document, &mut parsed_data);
    parsed_data
}

/// Recursive function that will go through each child of each element until it reaches plaintext.
fn walk(node: &NodeRef, data: &mut ParsedData) {
    if let Some(element) = node.as_element() {
        let tag_name = element.name.local.to_string();

        if let Some(class_attr) = element.attributes.borrow().get("class") {
            data.classes.push(class_attr.to_string());
        }

        // TODO: The proper parsing of this is currently broken
        if tag_name == "img" {
            let attrs = element.attributes.borrow();
            let img_tag = format!(
                "<img {}>",
                attrs
                    .map
                    .iter()
                    .map(|(k, v)| format!("{:?}=\"{:?}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            data.text.push_str(&img_tag);
        }
    }

    if let Some(text) = node.as_text() {
        data.text.push_str(text.borrow().as_str());
    }

    for child in node.children() {
        walk(&child, data);
    }
}
