//! A tiny element tree over quick-xml 0.42. Office parts are small, and walking a tree is far
//! simpler than juggling a state machine for nested shapes, tables and groups.

use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesStart, Event};

use crate::error::Error;

#[derive(Debug, Default)]
pub(crate) struct Element {
    /// The local name: `a:t` is `t`.
    pub name: String,
    /// Keys as written, prefix included: `r:embed`.
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

#[derive(Debug)]
pub(crate) enum Node {
    Element(Element),
    Text(String),
}

impl Element {
    /// An attribute without a prefix.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    /// A prefixed attribute by its local name, whatever the prefix (`r:id` for `id`).
    pub fn prefixed(&self, local: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k.split_once(':').is_some_and(|(_, l)| l == local)).map(|(_, v)| v.as_str())
    }

    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.elements().filter(move |e| e.name == name)
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.elements().find(|e| e.name == name)
    }

    /// Follows a chain of child names.
    pub fn path(&self, names: &[&str]) -> Option<&Element> {
        names.iter().try_fold(self, |at, name| at.child(name))
    }

    /// Every descendant with this name, depth first.
    pub fn descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Element>) {
        for e in self.elements() {
            if e.name == name {
                out.push(e);
            }
            e.descendants(name, out);
        }
    }

    /// All text below this element.
    pub fn text(&self) -> String {
        let mut out = String::new();
        self.push_text(&mut out);
        out
    }

    fn push_text(&self, out: &mut String) {
        for n in &self.children {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Element(e) => e.push_text(out),
            }
        }
    }
}

fn element(start: &BytesStart<'_>) -> Element {
    let mut attrs = Vec::new();
    for attribute in start.attributes().flatten() {
        let key = attribute.key.as_ref().to_string();
        let value = attribute.normalized_value(quick_xml::XmlVersion::Implicit1_0).map(|v| v.into_owned()).unwrap_or_else(|_| attribute.value.to_string());
        attrs.push((key, value));
    }
    Element { name: start.local_name().as_ref().to_string(), attrs, children: Vec::new() }
}

fn push_text(parent: &mut Element, text: &str) {
    if let Some(Node::Text(last)) = parent.children.last_mut() {
        last.push_str(text);
    } else {
        parent.children.push(Node::Text(text.to_string()));
    }
}

/// Parses a whole part and returns its root element.
pub(crate) fn parse(bytes: &[u8]) -> Result<Element, Error> {
    let xml = String::from_utf8_lossy(bytes);
    let xml = xml.trim_start_matches('\u{feff}');
    let mut reader = Reader::from_str(xml);
    let mut stack = vec![Element::default()];
    loop {
        let event = reader.read_event().map_err(|e| Error::Xml(e.to_string()))?;
        match event {
            Event::Start(start) => stack.push(element(&start)),
            Event::Empty(start) => {
                let e = element(&start);
                if let Some(top) = stack.last_mut() {
                    top.children.push(Node::Element(e));
                }
            }
            Event::End(_) => {
                if stack.len() < 2 {
                    return Err(Error::Xml("closing tag without an opening tag".into()));
                }
                if let Some(done) = stack.pop()
                    && let Some(top) = stack.last_mut()
                {
                    top.children.push(Node::Element(done));
                }
            }
            Event::Text(t) => {
                if let Some(top) = stack.last_mut() {
                    push_text(top, &t);
                }
            }
            Event::CData(t) => {
                if let Some(top) = stack.last_mut() {
                    push_text(top, &t);
                }
            }
            Event::GeneralRef(entity) => {
                let mut buf = [0u8; 4];
                let text: &str = match entity.resolve_char_ref() {
                    Ok(Some(c)) => c.encode_utf8(&mut buf),
                    _ => resolve_predefined_entity(&entity).unwrap_or(""),
                };
                if let Some(top) = stack.last_mut() {
                    push_text(top, text);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err(Error::Xml("the document ends inside an element".into()));
    }
    let root = stack.pop().unwrap_or_default();
    root.children
        .into_iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e),
            Node::Text(_) => None,
        })
        .ok_or_else(|| Error::Xml("no root element".into()))
}
