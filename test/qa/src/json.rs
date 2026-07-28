//! A minimal JSON reader for tests.
//!
//! The Engine emits JSON; a test that checked it with string matching would
//! only prove the text looks right. This parses it independently - no
//! dependency, no shared code with the emitter - so "parsable" means parsed.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonError {
    pub message: String,
    pub at: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.at)
    }
}

impl Json {
    pub fn parse(text: &str) -> Result<Self, JsonError> {
        let bytes = text.as_bytes();
        let mut at = 0;
        let value = parse_value(bytes, &mut at)?;
        skip_space(bytes, &mut at);
        if at != bytes.len() {
            return Err(error("trailing input", at));
        }
        Ok(value)
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Self::Object(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// The string field `key` of an object, or `None`.
    pub fn field(&self, key: &str) -> Option<&str> {
        self.as_object()?.get(key)?.as_str()
    }
}

fn error(message: &str, at: usize) -> JsonError {
    JsonError {
        message: message.to_owned(),
        at,
    }
}

fn skip_space(bytes: &[u8], at: &mut usize) {
    while *at < bytes.len() && matches!(bytes[*at], b' ' | b'\t' | b'\n' | b'\r') {
        *at += 1;
    }
}

fn parse_value(bytes: &[u8], at: &mut usize) -> Result<Json, JsonError> {
    skip_space(bytes, at);
    let Some(byte) = bytes.get(*at) else {
        return Err(error("unexpected end of input", *at));
    };
    match byte {
        b'{' => parse_object(bytes, at),
        b'[' => parse_array(bytes, at),
        b'"' => parse_string(bytes, at).map(Json::String),
        b't' => literal(bytes, at, "true", Json::Bool(true)),
        b'f' => literal(bytes, at, "false", Json::Bool(false)),
        b'n' => literal(bytes, at, "null", Json::Null),
        _ => parse_number(bytes, at),
    }
}

fn literal(bytes: &[u8], at: &mut usize, word: &str, value: Json) -> Result<Json, JsonError> {
    if bytes[*at..].starts_with(word.as_bytes()) {
        *at += word.len();
        Ok(value)
    } else {
        Err(error("unknown literal", *at))
    }
}

fn parse_number(bytes: &[u8], at: &mut usize) -> Result<Json, JsonError> {
    let start = *at;
    while *at < bytes.len() && matches!(bytes[*at], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
    {
        *at += 1;
    }
    std::str::from_utf8(&bytes[start..*at])
        .ok()
        .and_then(|text| text.parse::<f64>().ok())
        .map(Json::Number)
        .ok_or_else(|| error("invalid number", start))
}

fn parse_string(bytes: &[u8], at: &mut usize) -> Result<String, JsonError> {
    if bytes.get(*at) != Some(&b'"') {
        return Err(error("expected a string", *at));
    }
    *at += 1;
    let mut out = String::new();
    loop {
        let Some(byte) = bytes.get(*at).copied() else {
            return Err(error("unterminated string", *at));
        };
        *at += 1;
        match byte {
            b'"' => return Ok(out),
            b'\\' => {
                let Some(escape) = bytes.get(*at).copied() else {
                    return Err(error("unterminated escape", *at));
                };
                *at += 1;
                match escape {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = bytes
                            .get(*at..*at + 4)
                            .and_then(|slice| std::str::from_utf8(slice).ok())
                            .ok_or_else(|| error("truncated \\u escape", *at))?;
                        let code = u32::from_str_radix(hex, 16)
                            .map_err(|_| error("invalid \\u escape", *at))?;
                        out.push(char::from_u32(code).ok_or_else(|| error("invalid scalar", *at))?);
                        *at += 4;
                    }
                    _ => return Err(error("unknown escape", *at)),
                }
            }
            // Control characters must be escaped in JSON.
            0x00..=0x1f => return Err(error("unescaped control character", *at)),
            _ => {
                let start = *at - 1;
                let width = utf8_width(byte);
                let slice = bytes
                    .get(start..start + width)
                    .ok_or_else(|| error("truncated UTF-8", start))?;
                out.push_str(
                    std::str::from_utf8(slice).map_err(|_| error("invalid UTF-8", start))?,
                );
                *at = start + width;
            }
        }
    }
}

fn utf8_width(byte: u8) -> usize {
    match byte {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

fn parse_array(bytes: &[u8], at: &mut usize) -> Result<Json, JsonError> {
    *at += 1;
    let mut items = Vec::new();
    skip_space(bytes, at);
    if bytes.get(*at) == Some(&b']') {
        *at += 1;
        return Ok(Json::Array(items));
    }
    loop {
        items.push(parse_value(bytes, at)?);
        skip_space(bytes, at);
        match bytes.get(*at) {
            Some(b',') => *at += 1,
            Some(b']') => {
                *at += 1;
                return Ok(Json::Array(items));
            }
            _ => return Err(error("expected , or ]", *at)),
        }
    }
}

fn parse_object(bytes: &[u8], at: &mut usize) -> Result<Json, JsonError> {
    *at += 1;
    let mut fields = BTreeMap::new();
    skip_space(bytes, at);
    if bytes.get(*at) == Some(&b'}') {
        *at += 1;
        return Ok(Json::Object(fields));
    }
    loop {
        skip_space(bytes, at);
        let key = parse_string(bytes, at)?;
        skip_space(bytes, at);
        if bytes.get(*at) != Some(&b':') {
            return Err(error("expected :", *at));
        }
        *at += 1;
        let value = parse_value(bytes, at)?;
        fields.insert(key, value);
        skip_space(bytes, at);
        match bytes.get(*at) {
            Some(b',') => *at += 1,
            Some(b'}') => {
                *at += 1;
                return Ok(Json::Object(fields));
            }
            _ => return Err(error("expected , or }", *at)),
        }
    }
}
