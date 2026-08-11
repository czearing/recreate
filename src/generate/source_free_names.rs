use std::collections::BTreeSet;

/// The names a markup fragment mentions and does not itself bind.
///
/// Relocating a fragment into a new module is sound only when every one of these is
/// re-established at the destination — by import, by prop, or by re-declaration. This is the
/// one place that question is answered, so a stage that moves code cannot be written without
/// asking it, and no stage has to keep its own list of the shapes that carry a free name.
///
/// The set is deliberately an over-approximation. It authorises a refusal, so a name reported
/// in error costs one fragment that stays inline, while a name missed is a module that throws
/// the first time it renders. Anything the scan cannot classify is therefore reported. Only
/// subtraction has to be exact: every name removed below is a claim that the fragment binds it.
pub fn free_names(fragment: &str) -> BTreeSet<String> {
    let mut names = tags(fragment);
    let mut bound = BTreeSet::new();
    for expression in expressions(fragment) {
        read_expression(&expression, &mut names, &mut bound);
    }
    names.retain(|name| !bound.contains(name) && !ambient(name));
    names
}

/// A capitalised tag is a scope reference; a lowercase one is an intrinsic element and is never
/// looked up. Of a member tag only the leftmost segment is a name, the rest being properties.
fn tags(fragment: &str) -> BTreeSet<String> {
    let bytes = fragment.as_bytes();
    let mut names = BTreeSet::new();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'<' && bytes[index + 1].is_ascii_uppercase() {
            let start = index + 1;
            index = identifier_end(bytes, start);
            names.insert(fragment[start..index].to_string());
        } else {
            index += 1;
        }
    }
    names
}

/// The bodies of the `{ }` regions, which is where a fragment carries code rather than markup.
fn expressions(fragment: &str) -> Vec<String> {
    let bytes = fragment.as_bytes();
    let mut regions = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' | b'\'' | b'`' => index = string_end(bytes, index),
            b'{' => {
                if depth == 0 {
                    start = index + 1;
                }
                depth += 1;
                index += 1;
            }
            b'}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    regions.push(fragment[start..index].to_string());
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    regions
}

/// Collects the identifiers an expression reads, and separately the ones it binds itself.
fn read_expression(source: &str, names: &mut BTreeSet<String>, bound: &mut BTreeSet<String>) {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'"' | b'\'' | b'`') {
            index = string_end(bytes, index);
            continue;
        }
        if !starts_identifier(bytes[index]) {
            index += 1;
            continue;
        }
        let start = index;
        index = identifier_end(bytes, index);
        let name = &source[start..index];
        if arrow_follows(source, index) {
            bound.insert(name.to_string());
        }
        if member_read(bytes, start) || object_key(source, index) {
            continue;
        }
        names.insert(name.to_string());
    }
    for parameters in parameter_lists(source) {
        bound.extend(parameters);
    }
}

/// The names in a parenthesised parameter list, which the arrow that follows it binds.
fn parameter_lists(source: &str) -> Vec<Vec<String>> {
    let mut lists = Vec::new();
    let mut open = Vec::new();
    for (index, byte) in source.bytes().enumerate() {
        match byte {
            b'(' => open.push(index),
            b')' => {
                if let Some(start) = open.pop()
                    && arrow_follows(source, index + 1)
                {
                    lists.push(
                        source[start + 1..index]
                            .split(',')
                            .map(|name| name.trim().to_string())
                            .filter(|name| !name.is_empty())
                            .collect(),
                    );
                }
            }
            _ => {}
        }
    }
    lists
}

fn arrow_follows(source: &str, index: usize) -> bool {
    source[index..].trim_start().starts_with("=>")
}

/// A name reached through a `.` belongs to the object, not to the surrounding scope.
fn member_read(bytes: &[u8], start: usize) -> bool {
    bytes[..start]
        .iter()
        .rev()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| *byte == b'.')
}

/// A name followed by `:` is an object key, and a key names no binding.
fn object_key(source: &str, end: usize) -> bool {
    let rest = source[end..].trim_start();
    rest.starts_with(':') && !rest.starts_with("::")
}

/// Names every module already has, which no destination has to re-establish.
fn ambient(name: &str) -> bool {
    matches!(
        name,
        "true"
            | "false"
            | "null"
            | "undefined"
            | "new"
            | "typeof"
            | "void"
            | "in"
            | "of"
            | "return"
            | "React"
            | "document"
            | "window"
            | "Math"
            | "JSON"
            | "Object"
            | "Array"
            | "String"
            | "Number"
            | "Boolean"
            | "console"
    ) || name.bytes().all(|byte| byte.is_ascii_digit())
}

fn starts_identifier(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$'
}

fn identifier_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'_' | b'$'))
    {
        end += 1;
    }
    end
}

fn string_end(bytes: &[u8], start: usize) -> usize {
    let quote = bytes[start];
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            byte if byte == quote => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

#[cfg(test)]
#[path = "source_free_names_tests.rs"]
mod tests;
