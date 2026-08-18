//! A generated file imports exactly what it uses.
//!
//! The preamble each split file needs depends on what its own segment happens to contain, so
//! any preamble written as a fixed string is right for some files and wrong for the rest. That
//! is not cosmetic: an unused import names a module the reader will open and study, and it
//! keeps a runtime file alive that no code path can reach, so the recreation reads as though
//! it has behaviour it does not have. The mirror failure is worse: a file that uses a binding
//! it never imported does not parse at all.
//!
//! Rather than deciding per binding at each of the many places a source file is written, the
//! decision is made once, here, over the finished tree — which is also the only place that can
//! see a file after every writer has contributed to it, and the only place that knows how deep
//! in the tree each file sits, which is what a relative import path is made of.

use anyhow::Result;
use std::{fs, path::Path};

/// Bindings a writer may emit without arranging for them, because the import they need is a
/// path only the finished tree can spell. Keyed on the binding, so the writer names the
/// component and nothing else.
const SUPPLIED: [(&str, &str); 1] = [(super::shadow_root::COMPONENT, super::shadow_root::MODULE)];

/// Rewrites every emitted script under `root` so its imports match what it mentions.
pub fn prune_tree(root: &Path) -> Result<()> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(&path)?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if !matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("jsx" | "mjs" | "js")
            ) {
                continue;
            }
            let source = fs::read_to_string(&path)?;
            let supplied = supply(&source, root, &path);
            let source = supplied.as_deref().unwrap_or(&source);
            match prune(source) {
                Some(pruned) => fs::write(&path, pruned)?,
                None if supplied.is_some() => fs::write(&path, source)?,
                None => {}
            }
        }
    }
    Ok(())
}

/// The source with any supplied binding it mentions but does not bind now imported, or `None`
/// when it already binds everything it uses.
///
/// The module that defines a binding is skipped, or it would import itself.
fn supply(source: &str, root: &Path, path: &Path) -> Option<String> {
    let added = SUPPLIED
        .iter()
        .filter(|(_, module)| !path.ends_with(module))
        .filter(|(name, _)| mentions(source, name))
        .filter(|(name, _)| !source.lines().any(|line| is_import_of(line, name)))
        .map(|(name, module)| format!("import {{{name}}} from '{}{module}';", up(root, path)))
        .collect::<Vec<_>>();
    (!added.is_empty()).then(|| format!("{}\n{source}", added.join("\n")))
}

/// The prefix that reaches `root` from the directory `path` sits in.
fn up(root: &Path, path: &Path) -> String {
    let depth = path
        .parent()
        .and_then(|parent| parent.strip_prefix(root).ok())
        .map_or(0, |relative| relative.components().count());
    if depth == 0 {
        "./".to_string()
    } else {
        "../".repeat(depth)
    }
}

fn is_import_of(line: &str, name: &str) -> bool {
    is_named_import(line)
        && line
            .split_once('{')
            .and_then(|(_, rest)| rest.split_once('}'))
            .is_some_and(|(names, _)| names.split(',').any(|value| binding(value) == name))
}

/// The rewritten source, or `None` when every import is already used.
fn prune(source: &str) -> Option<String> {
    let body = source
        .lines()
        .filter(|line| !is_named_import(line))
        .collect::<Vec<_>>()
        .join("\n");
    let mut changed = false;
    let mut output = Vec::new();
    for line in source.lines() {
        if !is_named_import(line) {
            output.push(line.to_owned());
            continue;
        }
        let (head, rest) = line.split_once('{').unwrap();
        let (names, tail) = rest.split_once('}').unwrap();
        let kept = names
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .filter(|name| mentions(&body, binding(name)))
            .collect::<Vec<_>>();
        if kept.len()
            == names
                .split(',')
                .filter(|name| !name.trim().is_empty())
                .count()
        {
            output.push(line.to_owned());
            continue;
        }
        changed = true;
        if !kept.is_empty() {
            output.push(format!("{head}{{{}}}{tail}", kept.join(",")));
        }
    }
    changed.then(|| format!("{}\n", output.join("\n").trim_end()))
}

fn is_named_import(line: &str) -> bool {
    line.trim_start().starts_with("import ") && line.contains('{') && line.contains('}')
}

/// The local name an import clause introduces, which is what the file has to mention.
fn binding(name: &str) -> &str {
    name.rsplit(" as ").next().unwrap_or(name).trim()
}

/// Whether `source` uses `name` as an identifier rather than as part of a longer one. Shared
/// with the writers that compose a preamble, so "is this binding used" has one meaning.
pub(super) fn mentions(source: &str, name: &str) -> bool {
    let boundary = |value: Option<char>| {
        value.is_none_or(|character| {
            !character.is_alphanumeric() && character != '_' && character != '$'
        })
    };
    source.match_indices(name).any(|(index, _)| {
        boundary(source[..index].chars().next_back())
            && boundary(source[index + name.len()..].chars().next())
    })
}

#[cfg(test)]
#[path = "source_imports_tests.rs"]
mod tests;
