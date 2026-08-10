pub fn format_css(source: &str) -> String {
    let mut output = String::new();
    let mut indent = 0_usize;
    let mut quote = None;
    let mut comment = false;
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if comment {
            output.push(character);
            if character == '*' && characters.peek() == Some(&'/') {
                output.push('/');
                characters.next();
                comment = false;
            }
        } else if quote.is_none() && character == '/' && characters.peek() == Some(&'*') {
            output.push_str("/*");
            characters.next();
            comment = true;
        } else if let Some(active) = quote {
            output.push(character);
            if character == active && !output.ends_with(&format!("\\{active}")) {
                quote = None;
            }
        } else if matches!(character, '"' | '\'') {
            quote = Some(character);
            output.push(character);
        } else if character == '{' {
            trim_end(&mut output);
            output.push_str(" {\n");
            indent += 1;
            output.push_str(&"  ".repeat(indent));
        } else if character == '}' {
            indent = indent.saturating_sub(1);
            trim_end(&mut output);
            output.push('\n');
            output.push_str(&"  ".repeat(indent));
            output.push_str("}\n");
            output.push_str(&"  ".repeat(indent));
        } else if character == ';' {
            trim_end(&mut output);
            output.push_str(";\n");
            output.push_str(&"  ".repeat(indent));
        } else if character.is_whitespace() {
            if !output.ends_with([' ', '\n']) {
                output.push(' ');
            }
        } else {
            output.push(character);
        }
    }
    format!("{}\n", output.trim())
}

fn trim_end(output: &mut String) {
    output.truncate(output.trim_end().len());
}

pub fn css_classes(source: &str) -> Vec<&str> {
    let start = if source.starts_with('.') {
        0
    } else if let Some(start) = source.rfind("{.") {
        start + 1
    } else {
        return Vec::new();
    };
    let Some(open) = source[start..].find('{').map(|open| start + open) else {
        return Vec::new();
    };
    let mut classes = Vec::new();
    let mut remaining = &source[start..open];
    while let Some(index) = remaining.find('.') {
        remaining = &remaining[index + 1..];
        let length = remaining
            .chars()
            .take_while(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            })
            .map(char::len_utf8)
            .sum();
        if length == 0 {
            continue;
        }
        classes.push(&remaining[..length]);
        remaining = &remaining[length..];
    }
    classes
}

pub fn brace_delta(source: &str) -> i32 {
    source.chars().fold(0, |depth, character| match character {
        '{' => depth + 1,
        '}' => depth - 1,
        _ => depth,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_css_declarations() {
        assert_eq!(
            super::format_css(".a{color:red;background:white;}"),
            ".a {\n  color:red;\n  background:white;\n}\n"
        );
    }
}
