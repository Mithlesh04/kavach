// Copyright (c) 2026 Mithlesh and the Kavach contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://kavachlang.com | https://github.com/Mithlesh04/kavach

//! A small XML reader: the part of XML that SVD files use. Elements with
//! attributes, text, comments, `<?…?>` and `<!DOCTYPE …>` (skipped),
//! CDATA, and the five named entities with numbered ones. No namespaces
//! beyond keeping `a:b` names as they are, and no DTD.

/// One element: its name, attributes, child elements and text.
#[derive(Debug, Default)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    pub text: String,
}

impl Element {
    /// The first child named `name`.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    /// Every child named `name`.
    pub fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |c| c.name == name)
    }

    /// The text of the first child named `name`, with its spaces and
    /// line breaks collapsed to single spaces.
    pub fn text_of(&self, name: &str) -> Option<String> {
        let words: Vec<&str> = self.child(name)?.text.split_whitespace().collect();
        Some(words.join(" "))
    }

    /// The attribute `name`.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// The document's root element, or what is wrong and on which line.
pub fn parse(src: &str) -> Result<Element, String> {
    let mut r = Reader { src, at: 0 };
    r.misc()?;
    let root = r.element()?;
    r.misc()?;
    if r.at < src.len() {
        return Err(r.error("text after the root element"));
    }
    Ok(root)
}

struct Reader<'a> {
    src: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a str {
        &self.src[self.at..]
    }

    fn error(&self, what: &str) -> String {
        let line = self.src[..self.at].matches('\n').count() + 1;
        format!("line {line}: {what}")
    }

    /// Skips past `end`, or fails with `what`.
    fn skip_past(&mut self, end: &str, what: &str) -> Result<(), String> {
        match self.rest().find(end) {
            Some(i) => {
                self.at += i + end.len();
                Ok(())
            }
            None => Err(self.error(what)),
        }
    }

    /// Spaces, comments, `<?…?>` and `<!DOCTYPE …>` between elements.
    fn misc(&mut self) -> Result<(), String> {
        loop {
            let trimmed = self.rest().trim_start();
            self.at = self.src.len() - trimmed.len();
            if trimmed.starts_with("<!--") {
                self.skip_past("-->", "a comment that doesn't end")?;
            } else if trimmed.starts_with("<?") {
                self.skip_past("?>", "a `<?` that doesn't end")?;
            } else if trimmed.starts_with("<!DOCTYPE") {
                self.skip_past(">", "a `<!DOCTYPE` that doesn't end")?;
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> Result<String, String> {
        let end = self
            .rest()
            .find(|c: char| c.is_whitespace() || matches!(c, '>' | '/' | '='))
            .unwrap_or(self.rest().len());
        if end == 0 {
            return Err(self.error("a name was expected"));
        }
        let name = self.rest()[..end].to_string();
        self.at += end;
        Ok(name)
    }

    fn spaces(&mut self) {
        let trimmed = self.rest().trim_start();
        self.at = self.src.len() - trimmed.len();
    }

    fn element(&mut self) -> Result<Element, String> {
        if !self.rest().starts_with('<') {
            return Err(self.error("an element was expected"));
        }
        self.at += 1;
        let mut e = Element {
            name: self.name()?,
            ..Element::default()
        };
        loop {
            self.spaces();
            if self.rest().starts_with("/>") {
                self.at += 2;
                return Ok(e);
            }
            if self.rest().starts_with('>') {
                self.at += 1;
                break;
            }
            let key = self.name()?;
            self.spaces();
            if !self.rest().starts_with('=') {
                return Err(self.error(&format!("attribute `{key}` has no value")));
            }
            self.at += 1;
            self.spaces();
            let quote = match self.rest().chars().next() {
                Some(q @ ('"' | '\'')) => q,
                _ => return Err(self.error("an attribute's value must be in quotes")),
            };
            self.at += 1;
            let Some(end) = self.rest().find(quote) else {
                return Err(self.error("an attribute's value doesn't end"));
            };
            let value = unescape(&self.rest()[..end]).map_err(|w| self.error(&w))?;
            self.at += end + 1;
            e.attrs.push((key, value));
        }
        self.content(&mut e)?;
        Ok(e)
    }

    /// What is inside `e`, up to and with its end tag.
    fn content(&mut self, e: &mut Element) -> Result<(), String> {
        loop {
            let Some(lt) = self.rest().find('<') else {
                return Err(self.error(&format!("`<{}>` doesn't end", e.name)));
            };
            let text = unescape(&self.rest()[..lt]).map_err(|w| self.error(&w))?;
            e.text.push_str(&text);
            self.at += lt;
            let rest = self.rest();
            if let Some(after) = rest.strip_prefix("</") {
                let end = after.find('>').unwrap_or(after.len());
                if after[..end].trim() != e.name {
                    return Err(self.error(&format!("`</{}>` was expected", e.name)));
                }
                self.at += 2 + end + 1;
                return Ok(());
            } else if rest.starts_with("<!--") {
                self.skip_past("-->", "a comment that doesn't end")?;
            } else if let Some(after) = rest.strip_prefix("<![CDATA[") {
                let Some(end) = after.find("]]>") else {
                    return Err(self.error("CDATA that doesn't end"));
                };
                e.text.push_str(&after[..end]);
                self.at += 9 + end + 3;
            } else if rest.starts_with("<?") {
                self.skip_past("?>", "a `<?` that doesn't end")?;
            } else {
                let child = self.element()?;
                e.children.push(child);
            }
        }
    }
}

/// `text` with its entities (`&amp;`, `&#65;`, …) replaced.
fn unescape(text: &str) -> Result<String, String> {
    if !text.contains('&') {
        return Ok(text.to_string());
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let Some(end) = rest[i..].find(';') else {
            return Err("an `&` without its `;`".into());
        };
        let name = &rest[i + 1..i + end];
        let c = match name {
            "lt" => '<',
            "gt" => '>',
            "amp" => '&',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let n = match name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => name.strip_prefix('#').and_then(|d| d.parse().ok()),
                };
                n.and_then(char::from_u32)
                    .ok_or_else(|| format!("unknown entity `&{name};`"))?
            }
        };
        out.push(c);
        rest = &rest[i + end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_elements_attributes_text_and_entities() {
        let src = "<?xml version=\"1.0\"?>\n<!-- a chip -->\n<device schemaVersion='1.1'>\n  <name>A&amp;B &#65;&#x42;</name>\n  <empty/>\n  <d><![CDATA[x < y]]></d>\n</device>\n";
        let root = parse(src).unwrap();
        assert_eq!(root.name, "device");
        assert_eq!(root.attr("schemaVersion"), Some("1.1"));
        assert_eq!(root.text_of("name").as_deref(), Some("A&B AB"));
        assert!(root.child("empty").is_some());
        assert_eq!(root.text_of("d").as_deref(), Some("x < y"));
    }

    #[test]
    fn says_what_is_wrong_and_where() {
        for (src, said) in [
            ("<a>\n<b></a>", "line 2: `</b>` was expected"),
            (
                "<a x=1></a>",
                "line 1: an attribute's value must be in quotes",
            ),
            ("<a>&nope;</a>", "line 1: unknown entity `&nope;`"),
            ("<a></a>\n<b/>", "line 2: text after the root element"),
            ("<a>", "line 1: `<a>` doesn't end"),
        ] {
            assert_eq!(parse(src).unwrap_err(), said, "{src}");
        }
    }
}
