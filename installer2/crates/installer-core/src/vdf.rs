//! Small, strict KeyValues reader. Byte ranges allow edits without reformatting Steam files.
use anyhow::{Result, bail, ensure};
use std::ops::Range;

#[derive(Debug)]
pub enum Value {
    Text(String, Range<usize>),
    Object(Vec<Entry>),
}

#[derive(Debug)]
pub struct Entry {
    pub key: String,
    pub value: Value,
}

pub fn parse(input: &str) -> Result<Vec<Entry>> {
    let mut reader = Reader { input, offset: 0 };
    reader.object(false, 0)
}

pub fn object_at<'a>(entries: &'a [Entry], path: &[&str]) -> Option<&'a [Entry]> {
    if path.is_empty() {
        return Some(entries);
    }
    let entry = entries
        .iter()
        .find(|e| e.key.eq_ignore_ascii_case(path[0]))?;
    match &entry.value {
        Value::Object(children) => object_at(children, &path[1..]),
        _ => None,
    }
}

pub fn text_at<'a>(entries: &'a [Entry], key: &str) -> Option<&'a str> {
    entries.iter().find_map(|entry| match &entry.value {
        Value::Text(text, _) if entry.key.eq_ignore_ascii_case(key) => Some(text.as_str()),
        _ => None,
    })
}

struct Reader<'a> {
    input: &'a str,
    offset: usize,
}

impl Reader<'_> {
    fn whitespace(&mut self) {
        loop {
            while self
                .input
                .as_bytes()
                .get(self.offset)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.offset += 1;
            }
            if self.input[self.offset..].starts_with("//") {
                self.offset += self.input[self.offset..]
                    .find('\n')
                    .unwrap_or(self.input.len() - self.offset);
            } else {
                break;
            }
        }
    }

    fn string(&mut self) -> Result<(String, Range<usize>)> {
        self.whitespace();
        let start = self.offset;
        ensure!(
            self.input.as_bytes().get(start) == Some(&b'"'),
            "Expected quoted VDF string at byte {start}"
        );
        self.offset += 1;
        let mut value = String::new();
        while let Some(ch) = self.input[self.offset..].chars().next() {
            self.offset += ch.len_utf8();
            match ch {
                '"' => return Ok((value, start..self.offset)),
                '\\' => {
                    let next = self.input[self.offset..]
                        .chars()
                        .next()
                        .ok_or_else(|| anyhow::anyhow!("Incomplete VDF escape"))?;
                    self.offset += next.len_utf8();
                    match next {
                        '\\' | '"' => value.push(next),
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        _ => {
                            value.push('\\');
                            value.push(next);
                        }
                    }
                }
                _ => value.push(ch),
            }
        }
        bail!("Unclosed VDF string")
    }

    fn object(&mut self, nested: bool, depth: usize) -> Result<Vec<Entry>> {
        ensure!(depth < 64, "VDF nesting is too deep");
        let mut entries = Vec::new();
        loop {
            self.whitespace();
            match self.input.as_bytes().get(self.offset) {
                None => {
                    ensure!(!nested, "Unclosed VDF object");
                    return Ok(entries);
                }
                Some(b'}') => {
                    ensure!(nested, "Unexpected VDF closing brace");
                    self.offset += 1;
                    return Ok(entries);
                }
                _ => {}
            }
            let (key, _) = self.string()?;
            self.whitespace();
            let value = if self.input.as_bytes().get(self.offset) == Some(&b'{') {
                self.offset += 1;
                Value::Object(self.object(true, depth + 1)?)
            } else {
                let (text, span) = self.string()?;
                Value::Text(text, span)
            };
            entries.push(Entry { key, value });
        }
    }
}

pub fn quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    )
}
