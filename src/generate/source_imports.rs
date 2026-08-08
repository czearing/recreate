//! A generated file imports only what it uses.
//!
//! The preamble each split file needs depends on what its own segment happens to contain, so
//! any preamble written as a fixed string is right for some files and wrong for the rest. That
//! is not cosmetic: an unused import names a module the reader will open and study, and it
//! keeps a runtime file alive that no code path can reach, so the recreation reads as though
//! it has behaviour it does not have.
//!
//! Rather than deciding per binding at each of the many places a source file is written, the
//! decision is made once, here, over the finished tree — which is also the only place that can
//! see a file after every writer has contributed to it.

use anyhow::Result;
use std::{fs, path::Path};

/// Rewrites every emitted script under `root` to drop named imports it never mentions.
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
            if let Some(pruned) = prune(&source) {
                fs::write(&path, pruned)?;
            }
        }
    }
    Ok(())
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
mod tests {
    /// The case the corpus was shipping: a portal helper imported into every view, and used by
    /// two thirds of none of them.
    #[test]
    fn drops_an_import_the_file_never_mentions() {
        let source = "import React from 'react';\nimport {createPortal} from 'react-dom';\n\nexport default function View(){return <div/>}\n";
        assert_eq!(
            super::prune(source).unwrap(),
            "import React from 'react';\n\nexport default function View(){return <div/>}\n"
        );
    }

    /// Pruning must be a narrowing. A file that uses what it imports is returned untouched, and
    /// reported as untouched so the tree is not rewritten for nothing.
    #[test]
    fn leaves_a_used_import_alone() {
        let source = "import {createPortal} from 'react-dom';\nconst view=createPortal(null,document.body);\n";
        assert_eq!(super::prune(source), None);
    }

    /// One clause commonly carries several bindings, so the decision is per name.
    #[test]
    fn keeps_only_the_used_names_of_a_shared_clause() {
        let source = "import {keyActivate,ExistingSurface,InsertedSurface} from './runtime.jsx';\nconst a=<InsertedSurface/>;\n";
        assert_eq!(
            super::prune(source).unwrap(),
            "import {InsertedSurface} from './runtime.jsx';\nconst a=<InsertedSurface/>;\n"
        );
    }

    /// An import renamed on the way in is used under its local name, which is the one that has
    /// to appear.
    #[test]
    fn judges_a_renamed_import_by_its_local_name() {
        let used =
            "import {moveCarousel as move} from './carousel.mjs';\nconst next=move(state);\n";
        assert_eq!(super::prune(used), None);
        let unused = "import {moveCarousel as move} from './carousel.mjs';\nconst next=moveCarousel(state);\n";
        assert_eq!(
            super::prune(unused).unwrap(),
            "const next=moveCarousel(state);\n"
        );
    }

    /// A name that merely occurs inside a longer identifier is not a use of it, or nothing
    /// would ever be pruned.
    #[test]
    fn does_not_count_a_longer_identifier_as_a_use() {
        let source = "import {Surface} from './runtime.jsx';\nconst a=<SurfaceHost/>;\n";
        assert_eq!(super::prune(source).unwrap(), "const a=<SurfaceHost/>;\n");
    }

    /// An import used only by another import line is still unused by the file.
    #[test]
    fn ignores_uses_that_are_themselves_import_clauses() {
        let source = "import {createPortal} from 'react-dom';\nimport {createPortal} from './shim.js';\nconst a=1;\n";
        assert_eq!(super::prune(source).unwrap(), "const a=1;\n");
    }
}
