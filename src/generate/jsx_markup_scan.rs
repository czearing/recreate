//! The grammar half of reading generated JSX markup: where a name sits, not what it means.
//!
//! The input is never arbitrary JSX. It is what `jsx_render` and `jsx_attrs` emit, so this
//! scanner only has to share that writer's grammar — the same rule `generated_source`
//! follows for string literals, whose escaping it reuses.

pub(super) enum Value<'a> {
    /// A `serde_json` string literal, still escaped. Every attribute the generator writes
    /// carries one, apart from booleans.
    Literal(&'a str),
    /// Any other brace expression, such as the `{true}` a boolean attribute emits.
    Expression(&'a str),
}

pub(super) enum Token<'a> {
    Text(&'a str),
    Literal(&'a str),
    Open {
        closing: bool,
        name: &'a str,
    },
    Attribute {
        name: &'a str,
        value: Option<Value<'a>>,
    },
    Close {
        self_closing: bool,
    },
}

pub(super) fn scan<'a>(source: &'a str, mut emit: impl FnMut(Token<'a>)) {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'<' => index = tag(source, index, &mut emit),
            b'{' => {
                let (end, inner) = brace(source, index);
                emit(match literal_body(inner) {
                    Some(body) => Token::Literal(body),
                    None => Token::Text(inner),
                });
                index = end;
            }
            _ => {
                let start = index;
                while index < bytes.len() && !matches!(bytes[index], b'<' | b'{') {
                    index += 1;
                }
                emit(Token::Text(&source[start..index]));
            }
        }
    }
}

fn tag<'a>(source: &'a str, start: usize, emit: &mut impl FnMut(Token<'a>)) -> usize {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    let closing = bytes.get(index) == Some(&b'/');
    index += usize::from(closing);
    let name_start = index;
    while index < bytes.len() && is_name(bytes[index]) {
        index += 1;
    }
    emit(Token::Open {
        closing,
        name: &source[name_start..index],
    });
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        match bytes.get(index) {
            None => return index,
            Some(b'>') => {
                emit(Token::Close {
                    self_closing: false,
                });
                return index + 1;
            }
            Some(b'/') => {
                emit(Token::Close { self_closing: true });
                return index + usize::from(bytes.get(index + 1) == Some(&b'>')) + 1;
            }
            _ => {}
        }
        let name_start = index;
        while index < bytes.len() && is_name(bytes[index]) {
            index += 1;
        }
        if index == name_start {
            index += 1;
            continue;
        }
        let name = &source[name_start..index];
        let value = (bytes.get(index) == Some(&b'=')).then(|| {
            let (end, inner) = if bytes.get(index + 1) == Some(&b'{') {
                brace(source, index + 1)
            } else {
                let (end, body) = quoted(source, index + 1);
                index = end;
                return Value::Expression(body);
            };
            index = end;
            match literal_body(inner) {
                Some(body) => Value::Literal(body),
                None => Value::Expression(inner),
            }
        });
        emit(Token::Attribute { name, value });
    }
}

/// The extent of a brace expression, skipping braces that fall inside a string literal.
fn brace(source: &str, open: usize) -> (usize, &str) {
    let bytes = source.as_bytes();
    let mut index = open + 1;
    let mut depth = 1usize;
    while index < bytes.len() && depth > 0 {
        match bytes[index] {
            b'"' => index = quoted(source, index).0 - 1,
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    (
        index,
        &source[open + 1..index.saturating_sub(1).max(open + 1)],
    )
}

/// The extent of a `serde_json` string literal, whose escaping this shares with
/// `generated_source::string_literals`. Reading these by splitting on the quote character
/// would invert the parity of every literal after a quote in captured page text.
fn quoted(source: &str, open: usize) -> (usize, &str) {
    let bytes = source.as_bytes();
    let mut end = open + 1;
    while end < bytes.len() && bytes[end] != b'"' {
        end += if bytes[end] == b'\\' { 2 } else { 1 };
    }
    let end = end.min(bytes.len());
    (end + 1, &source[open + 1..end])
}

/// The escaped body of a brace expression that holds exactly one string literal.
fn literal_body(inner: &str) -> Option<&str> {
    let trimmed = inner.trim();
    (trimmed.starts_with('"') && quoted(trimmed, 0).0 == trimmed.len())
        .then(|| quoted(trimmed, 0).1)
}

fn is_name(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.')
}
