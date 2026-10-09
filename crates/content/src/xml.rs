//! Small read-only XML tree on top of quick-xml, enough for GPIF and MusicXML.
//! Namespaces are dropped (local names only); text and CDATA are joined per element.

use crate::error::ImportError;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

/// Nesting deeper than this is rejected (real files stay below 20).
const MAX_DEPTH: usize = 256;
const BOM: &[u8] = &[0xef, 0xbb, 0xbf];

#[derive(Debug, Clone, Default)]
pub(crate) struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Element>,
    text: String,
}

impl Element {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn children(&self) -> &[Element] {
        &self.children
    }

    pub(crate) fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    pub(crate) fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// Follows a path of child names, e.g. `["Staves", "Staff"]`.
    pub(crate) fn path(&self, names: &[&str]) -> Option<&Element> {
        names.iter().try_fold(self, |at, name| at.child(name))
    }

    pub(crate) fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    pub(crate) fn text(&self) -> &str {
        self.text.trim()
    }

    /// Trimmed text of child `name`.
    pub(crate) fn value(&self, name: &str) -> Option<&str> {
        self.child(name).map(Element::text)
    }

    /// Text of child `name` parsed as a number.
    pub(crate) fn number<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.value(name)?.parse().ok()
    }

    /// All descendants named `name`, depth first.
    pub(crate) fn every<'a>(&'a self, name: &str) -> Vec<&'a Element> {
        let mut found = Vec::new();
        let mut stack: Vec<&Element> = self.children.iter().rev().collect();
        while let Some(at) = stack.pop() {
            if at.name == name {
                found.push(at);
            }
            stack.extend(at.children.iter().rev());
        }
        found
    }

    /// First descendant named `name`, depth first.
    pub(crate) fn find(&self, name: &str) -> Option<&Element> {
        let mut stack: Vec<&Element> = vec![self];
        while let Some(at) = stack.pop() {
            if at.name == name && !std::ptr::eq(at, self) {
                return Some(at);
            }
            stack.extend(at.children.iter().rev());
        }
        None
    }
}

/// Parses a document and returns its root element.
pub(crate) fn parse(data: &[u8]) -> Result<Element, ImportError> {
    let data = data.strip_prefix(BOM).unwrap_or(data);
    let mut reader = Reader::from_reader(data);
    let mut stack: Vec<Element> = vec![Element::default()];

    loop {
        let event = reader
            .read_event()
            .map_err(|e| ImportError::Malformed(format!("XML: {e}")))?;
        match event {
            Event::Start(start) => {
                if stack.len() > MAX_DEPTH {
                    return Err(ImportError::bad("XML nested too deeply"));
                }
                stack.push(element(&start)?);
            }
            Event::Empty(start) => {
                let element = element(&start)?;
                top(&mut stack)?.children.push(element);
            }
            Event::End(_) => {
                let done = stack.pop().ok_or_else(|| ImportError::bad("XML end tag"))?;
                top(&mut stack)?.children.push(done);
            }
            Event::Text(text) => {
                let text = text
                    .unescape()
                    .map_err(|e| ImportError::Malformed(format!("XML text: {e}")))?;
                top(&mut stack)?.text.push_str(&text);
            }
            Event::CData(data) => {
                let text = String::from_utf8_lossy(&data);
                top(&mut stack)?.text.push_str(&text);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    // The document node holds exactly the root element.
    let document = stack
        .pop()
        .ok_or_else(|| ImportError::bad("XML document"))?;
    if !stack.is_empty() {
        return Err(ImportError::bad("XML ends inside an element"));
    }
    document
        .children
        .into_iter()
        .next()
        .ok_or_else(|| ImportError::bad("XML without root element"))
}

fn top(stack: &mut [Element]) -> Result<&mut Element, ImportError> {
    stack
        .last_mut()
        .ok_or_else(|| ImportError::bad("XML end tag without start"))
}

fn element(start: &BytesStart) -> Result<Element, ImportError> {
    let name = String::from_utf8_lossy(start.local_name().as_ref()).into_owned();
    let mut attrs = Vec::new();
    for attr in start.attributes() {
        let attr = attr.map_err(|e| ImportError::Malformed(format!("XML attribute: {e}")))?;
        let key = String::from_utf8_lossy(attr.key.local_name().as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map_err(|e| ImportError::Malformed(format!("XML attribute: {e}")))?;
        attrs.push((key, value.into_owned()));
    }
    Ok(Element {
        name,
        attrs,
        children: Vec::new(),
        text: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_tree() {
        let doc =
            br#"<?xml version="1.0"?><a x="1"><b>hi &amp; <![CDATA[<there>]]></b><c/><b>2</b></a>"#;
        let root = parse(doc).unwrap();
        assert_eq!(root.name(), "a");
        assert_eq!(root.attr("x"), Some("1"));
        assert_eq!(root.value("b"), Some("hi & <there>"));
        assert_eq!(root.all("b").count(), 2);
        assert!(root.find("c").is_some());
    }

    #[test]
    fn rejects_broken_xml() {
        assert!(parse(b"<a><b></a>").is_err());
        assert!(parse(b"<a>").is_err());
        assert!(parse(b"").is_err());
        let deep = "<a>".repeat(1000);
        assert!(parse(deep.as_bytes()).is_err());
    }
}
