use std::collections::HashMap;

pub fn compact_unique_generated(source: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let parsed = lines
        .iter()
        .map(|line| generated_rule(line))
        .collect::<Vec<_>>();
    let mut counts = HashMap::<(String, String, String), usize>::new();
    for rule in parsed.iter().flatten() {
        *counts
            .entry((
                rule.prefix.clone(),
                rule.selector.clone(),
                rule.suffix.clone(),
            ))
            .or_default() += 1;
    }
    let mut output = Vec::<Option<String>>::new();
    let mut previous_group: Option<(usize, (String, String, String))> = None;
    for (line, rule) in lines.into_iter().zip(parsed) {
        let Some(rule) = rule else {
            output.push(Some(line.to_string()));
            previous_group = None;
            continue;
        };
        let selector_key = (
            rule.prefix.clone(),
            rule.selector.clone(),
            rule.suffix.clone(),
        );
        if counts.get(&selector_key) != Some(&1) {
            output.push(Some(line.to_string()));
            previous_group = None;
            continue;
        }
        let group_key = (rule.prefix.clone(), rule.body.clone(), rule.suffix.clone());
        if let Some(index) = previous_group
            .as_ref()
            .filter(|(_, previous)| previous == &group_key)
            .map(|(index, _)| *index)
        {
            if let Some(existing) = &mut output[index] {
                let insertion = existing[rule.prefix.len()..]
                    .find('{')
                    .expect("generated rule selector should have an opening brace")
                    + rule.prefix.len();
                existing.insert_str(insertion, &format!(",{}", rule.selector));
            }
        } else {
            previous_group = Some((output.len(), group_key));
            output.push(Some(line.to_string()));
        }
    }
    format!(
        "{}\n",
        output.into_iter().flatten().collect::<Vec<_>>().join("\n")
    )
}

struct GeneratedRule {
    prefix: String,
    selector: String,
    body: String,
    suffix: String,
}

fn generated_rule(line: &str) -> Option<GeneratedRule> {
    if super::source_style_support::brace_delta(line) != 0 {
        return None;
    }
    let start = if line.starts_with('.') {
        0
    } else {
        line.rfind("{.")? + 1
    };
    let open = line[start..].find('{')? + start;
    let close = line[open + 1..].find('}')? + open + 1;
    let selector = line[start..open].trim();
    if selector.contains(',')
        || selector.contains(char::is_whitespace)
        || selector.contains('>')
        || !generated_selector(selector)
    {
        return None;
    }
    Some(GeneratedRule {
        prefix: line[..start].to_string(),
        selector: selector.to_string(),
        body: line[open + 1..close].to_string(),
        suffix: line[close + 1..].to_string(),
    })
}

fn generated_selector(selector: &str) -> bool {
    let Some(rest) = selector.strip_prefix('.') else {
        return false;
    };
    matches!(rest.as_bytes().first(), Some(b'r' | b's'))
        && rest[1..]
            .chars()
            .take(10)
            .all(|character| character.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    #[test]
    fn compacts_unique_generated_selectors_only() {
        let output = super::compact_unique_generated(
            ".r1234567890{color:red;}\n.s1234567890{color:red;}\n.a{color:red;}\n",
        );
        assert!(output.contains(".r1234567890,.s1234567890{color:red;}"));
        assert!(output.contains(".a{color:red;}"));
    }

    #[test]
    fn compacts_inside_media_without_changing_the_media_selector() {
        let output = super::compact_unique_generated(
            "@media (forced-colors: active){.r1234567890{color:red;}}\n@media (forced-colors: active){.s1234567890{color:red;}}\n",
        );
        assert!(
            output
                .contains("@media (forced-colors: active){.r1234567890,.s1234567890{color:red;}}")
        );
    }

    #[test]
    fn preserves_cascade_barriers() {
        let css = ".r1234567890{color:red;}\n.a{color:blue;}\n.s1234567890{color:red;}\n";
        assert_eq!(super::compact_unique_generated(css), css);
    }
}
