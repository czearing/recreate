pub fn css_brace_delta(line: &str) -> i32 {
    let mut quote = None;
    let mut escaped = false;
    let mut delta = 0;
    for character in line.chars() {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if let Some(active) = quote {
            if character == active {
                quote = None;
            }
        } else if matches!(character, '"' | '\'') {
            quote = Some(character);
        } else if character == '{' {
            delta += 1;
        } else if character == '}' {
            delta -= 1;
        }
    }
    delta
}

#[cfg(test)]
mod tests {
    use super::css_brace_delta;

    #[test]
    fn counts_only_structural_braces() {
        assert_eq!(css_brace_delta(".a{color:red;}"), 0);
        assert_eq!(css_brace_delta("@media(x){"), 1);
        assert_eq!(css_brace_delta("}"), -1);
    }

    #[test]
    fn ignores_braces_inside_quoted_values() {
        assert_eq!(css_brace_delta(".a{content:\"{\";}"), 0);
        assert_eq!(css_brace_delta(".a{content:'}';}"), 0);
    }
}
