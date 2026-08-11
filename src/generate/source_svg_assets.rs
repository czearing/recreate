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

pub(super) fn image(svg: &str, filename: &str) -> String {
    let attributes = ["className", "aria-hidden", "height", "width"]
        .into_iter()
        .filter_map(|name| {
            super::jsx_markup::root_attribute(svg, name)
                .map(|value| format!(" {name}={{{}}}", serde_json::to_string(&value).unwrap()))
        })
        .collect::<String>();
    format!("<img src={{\"/assets/{filename}\"}} alt={{\"\"}}{attributes} />")
}

/// The relocated graphic as a standalone document: the stylesheet it needs carried in, and
/// every namespace its own names use bound on its root.
pub(super) fn document(svg: &str, css: &str) -> String {
    let styles = super::css_closure::self_contained(css, &classes(svg));
    let names = super::jsx_markup::qualified_names(svg);
    let xml = super::xml_namespaces::declare(super::jsx_markup::to_xml(svg), &names);
    first_child(
        xml,
        &format!("<style>{styles}</style>"),
        &names.root_element,
    )
}

/// Inserts `child` as the root's first child. Inserting after the first `>` is only that
/// while the root has a content model: a self-closing root ends at `/>`, so the same offset
/// puts the stylesheet *beside* the root and produces a document with two top-level
/// elements, which no XML parser accepts. Giving the root a body is what makes it a parent.
fn first_child(mut xml: String, child: &str, element: &str) -> String {
    let Some(index) = xml.find('>') else {
        return xml;
    };
    match xml[..index].ends_with('/') {
        true => {
            let cut = xml[..index].trim_end_matches(['/', ' ']).len();
            xml.replace_range(cut..=index, &format!(">{child}</{element}>"));
            xml
        }
        false => {
            xml.insert_str(index + 1, child);
            xml
        }
    }
}

/// Every class the relocated subtree references, which is a genuinely subtree-wide
/// question: the asset's stylesheet is carved from what the whole graphic names, so a
/// descendant's class has to reach it. This is the one caller `attribute_values` is
/// written for; the stand-in `<img>` asks about a single element and uses
/// `root_attribute` instead.
fn classes(source: &str) -> Vec<String> {
    super::jsx_markup::attribute_values(source, "className")
        .iter()
        .flat_map(|value| value.split_whitespace().map(str::to_string))
        .collect()
}
