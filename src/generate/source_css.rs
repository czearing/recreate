use super::source_css_compact::css_brace_delta;
use std::collections::HashMap;

pub fn dedupe_exact(source: &str) -> String {
    let lines = source.lines().collect::<Vec<_>>();
    let mut depth = 0_i32;
    let mut last = HashMap::<&str, usize>::new();
    for (index, line) in lines.iter().enumerate() {
        let delta = css_brace_delta(line);
        if depth == 0 && delta == 0 {
            last.insert(line, index);
        }
        depth += delta;
    }
    depth = 0;
    let mut output = String::new();
    for (index, line) in lines.into_iter().enumerate() {
        let delta = css_brace_delta(line);
        if depth != 0 || delta != 0 || last.get(line) == Some(&index) {
            output.push_str(line);
            output.push('\n');
        }
        depth += delta;
    }
    output
}
