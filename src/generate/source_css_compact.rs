use std::collections::HashSet;

pub fn compact(source: &str) -> String {
    let mut entries = Vec::<Entry>::new();
    let mut exact = HashSet::new();
    let mut depth = 0_i32;
    let mut previous_group = None;
    for line in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let delta = css_brace_delta(line);
        if let Some(rule) = (depth == 0 && delta == 0)
            .then(|| simple_rule(line))
            .flatten()
        {
            let key = (rule.prefix.clone(), rule.body.clone(), rule.suffix.clone());
            if let Some(index) = previous_group
                .as_ref()
                .filter(|(_, previous)| previous == &key)
                .map(|(index, _)| *index)
            {
                if let Entry::Rule(existing) = &mut entries[index]
                    && !existing.selectors.contains(&rule.selectors[0])
                {
                    existing.selectors.push(rule.selectors[0].clone());
                }
            } else {
                previous_group = Some((entries.len(), key));
                entries.push(Entry::Rule(rule));
            }
        } else if depth != 0 || delta != 0 || exact.insert(line.to_string()) {
            entries.push(Entry::Raw(line.to_string()));
            previous_group = None;
        }
        depth += delta;
    }
    let mut output = entries
        .into_iter()
        .map(|entry| match entry {
            Entry::Raw(line) => line,
            Entry::Rule(rule) => format!(
                "{}{}{{{}}}{}",
                rule.prefix,
                rule.selectors.join(","),
                rule.body,
                rule.suffix
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

enum Entry {
    Raw(String),
    Rule(Rule),
}

struct Rule {
    prefix: String,
    selectors: Vec<String>,
    body: String,
    suffix: String,
}

fn simple_rule(line: &str) -> Option<Rule> {
    let start = if line.starts_with('.') {
        0
    } else {
        line.rfind("{.")? + 1
    };
    let open = line[start..].find('{')? + start;
    let selector = &line[start..open];
    if !selector.starts_with('.')
        || selector.contains(char::is_whitespace)
        || selector.contains(['>', '+', '~', '['])
    {
        return None;
    }
    let close = line[open + 1..].find('}')? + open + 1;
    let body = &line[open + 1..close];
    if body.contains('{') {
        return None;
    }
    Some(Rule {
        prefix: line[..start].to_string(),
        selectors: vec![selector.to_string()],
        body: body.to_string(),
        suffix: line[close + 1..].to_string(),
    })
}

pub fn rule_classes(line: &str) -> Option<Vec<String>> {
    let classes = simple_rule(line)?
        .selectors
        .iter()
        .flat_map(|selector| selector.split('.').skip(1))
        .map(|part| {
            part.chars()
                .take_while(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
                .collect::<String>()
        })
        .filter(|class_name| !class_name.is_empty())
        .collect::<Vec<_>>();
    (!classes.is_empty()).then_some(classes)
}

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
    #[test]
    fn combines_identical_rules_in_the_same_context() {
        let css = ".a{color:red;}\n.b{color:red;}\n@media(x){.c{color:red;}}\n";
        let output = super::compact(css);
        assert!(output.contains(".a,.b{color:red;}"));
        assert!(output.contains("@media(x){.c{color:red;}}"));
    }

    #[test]
    fn preserves_cascade_barriers() {
        let css = ".a{color:red;}\n.shared{color:blue;}\n.b{color:red;}\n";
        assert_eq!(super::compact(css), css);
    }
}
