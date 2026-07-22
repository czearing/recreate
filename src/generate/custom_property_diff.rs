use std::collections::BTreeMap;

pub fn against(base: &str, responsive: &str) -> String {
    let base = declarations(base);
    declarations(responsive)
        .into_iter()
        .filter(|(selector, value)| base.get(selector) != Some(value))
        .map(|(selector, value)| format!("{selector}{{{value}}}\n"))
        .collect()
}

fn declarations(css: &str) -> BTreeMap<String, String> {
    let mut rules = BTreeMap::<String, String>::new();
    for line in css.lines() {
        let Some((selector, value)) = line.split_once('{') else {
            continue;
        };
        let Some(value) = value.strip_suffix('}') else {
            continue;
        };
        rules.entry(selector.into()).or_default().push_str(value);
    }
    rules
}

#[cfg(test)]
mod tests {
    #[test]
    fn emits_only_responsive_declaration_changes() {
        let base = ":root{--brand:red;}\nmain{color:red;}\nmain{z-index:1;}\n";
        let responsive = ":root{--brand:blue;}\nmain{color:red;}\nmain{z-index:1;}\n";

        assert_eq!(super::against(base, responsive), ":root{--brand:blue;}\n");
    }
}
