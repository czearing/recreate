use super::source_dedupe_support::{normalize, replace_ranges, reusable_svg, svg_blocks};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub fn extract(sources: &mut [&mut String], directory: &Path, css: &str) -> Result<()> {
    fs::create_dir_all(directory)?;
    let mut assets = BTreeMap::<String, String>::new();
    for source in sources.iter() {
        for (_, _, svg) in svg_blocks(source) {
            if reusable_svg(&svg) {
                let normalized = normalize(&svg);
                assets.entry(normalized.clone()).or_insert_with(|| {
                    format!(
                        "{}.svg",
                        &hex::encode(Sha256::digest(normalized.as_bytes()))[..20]
                    )
                });
            }
        }
    }
    for (svg, filename) in &assets {
        fs::write(directory.join(filename), document(svg, css))?;
    }
    let mut encoded_assets = BTreeMap::<String, String>::new();
    for source in sources.iter() {
        for (_, _, svg) in encoded_svg_sources(source) {
            encoded_assets.entry(svg.clone()).or_insert_with(|| {
                format!("{}.svg", &hex::encode(Sha256::digest(svg.as_bytes()))[..20])
            });
        }
    }
    for (svg, filename) in &encoded_assets {
        fs::write(directory.join(filename), svg)?;
    }
    for source in sources {
        let mut replacements = svg_blocks(source)
            .into_iter()
            .filter_map(|(start, end, svg)| {
                let filename = assets.get(&normalize(&svg))?;
                Some((start, end, image(&svg, filename)))
            })
            .collect::<Vec<_>>();
        replace_ranges(source, &mut replacements);
        let mut replacements = encoded_svg_sources(source)
            .into_iter()
            .filter_map(|(start, end, svg)| {
                encoded_assets
                    .get(&svg)
                    .map(|filename| (start, end, format!("/assets/{filename}")))
            })
            .collect::<Vec<_>>();
        replace_ranges(source, &mut replacements);
    }

    Ok(())
}

pub(super) fn encoded_svg_sources(source: &str) -> Vec<(usize, usize, String)> {
    const MARKER: &str = "src={\"data:image/svg+xml;utf8,";
    let mut matches = Vec::new();
    let mut offset = 0;
    while let Some(relative) = source[offset..].find(MARKER) {
        let start = offset + relative + "src={\"".len();
        let Some(relative_end) = source[start..].find("\"}") else {
            break;
        };
        let end = start + relative_end;
        if let Some(comma) = source[start..end].find(',')
            && let Some(svg) = percent_decode(&source[start + comma + 1..end])
        {
            matches.push((start, end, svg));
        }
        offset = end;
    }
    matches
}

fn percent_decode(source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let value = u8::from_str_radix(&source[index + 1..index + 3], 16).ok()?;
            decoded.push(value);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn image(svg: &str, filename: &str) -> String {
    let attributes = ["className", "aria-hidden", "height", "width"]
        .into_iter()
        .filter_map(|name| {
            attribute(svg, name)
                .map(|value| format!(" {name}={{{}}}", serde_json::to_string(value).unwrap()))
        })
        .collect::<String>();
    format!("<img src={{\"/assets/{filename}\"}} alt={{\"\"}}{attributes} />")
}

pub(super) fn document(svg: &str, css: &str) -> String {
    let styles = super::css_closure::self_contained(css, &classes(svg));
    let mut svg = to_xml(svg);
    if !svg.contains("xmlns=") {
        svg = svg.replacen("<svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"", 1);
    }
    if let Some(index) = svg.find('>') {
        svg.insert_str(index + 1, &format!("<style>{styles}</style>"));
    }
    svg
}

fn classes(source: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut remaining = source;
    while let Some(index) = remaining.find("className={\"") {
        remaining = &remaining[index + 12..];
        let Some(end) = remaining.find("\"}") else {
            break;
        };
        classes.extend(remaining[..end].split_whitespace().map(str::to_string));
        remaining = &remaining[end + 2..];
    }
    classes
}

fn attribute<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let marker = format!("{name}={{\"");
    let start = source.find(&marker)? + marker.len();
    let end = source[start..].find("\"}")? + start;
    Some(&source[start..end])
}

pub(super) fn to_xml(source: &str) -> String {
    let mut output = source
        .replace("className={\"", "class=\"")
        .replace("={\"", "=\"")
        .replace("\"}", "\"");
    for (react, svg) in [
        ("fillOpacity", "fill-opacity"),
        ("strokeWidth", "stroke-width"),
        ("strokeLinecap", "stroke-linecap"),
        ("strokeLinejoin", "stroke-linejoin"),
        ("stopColor", "stop-color"),
        ("stopOpacity", "stop-opacity"),
        ("clipPath", "clip-path"),
        ("fillRule", "fill-rule"),
        ("clipRule", "clip-rule"),
    ] {
        output = output.replace(react, svg);
    }
    while let Some(start) = output.find("={") {
        let Some(relative_end) = output[start + 2..].find('}') else {
            break;
        };
        let end = start + 2 + relative_end;
        let value = output[start + 2..end].to_string();
        output.replace_range(start..=end, &format!("=\"{value}\""));
    }
    output
}
