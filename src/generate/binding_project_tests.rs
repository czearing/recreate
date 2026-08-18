use super::binding_check::unbound;
use super::project::write_project;
use super::project_test_support as support;
use std::fs;
use std::path::{Path, PathBuf};

/// The invariant, over the artifact it belongs to: every identifier the generated project
/// references resolves to a declaration in the same file, an import, or a global the runtime
/// actually provides.
///
/// The defect this closes is `App.jsx` referencing a bare `width` — from
/// `templates/tab_scroll.mjs`, where the name is free by construction and therefore invisible
/// to the per-fragment `node --check`, and again from the interaction overlay, which handed
/// `Interaction{n}` a width nothing had measured. Checking the assembled project instead
/// covers all eleven templates at once, and covers every template added later without anyone
/// remembering to extend a list.
#[tokio::test]
async fn generated_project_references_no_name_it_does_not_bind() {
    let directory = tempfile::tempdir().unwrap();
    write_project(&support::specification(), directory.path(), &[])
        .await
        .unwrap();
    let mut free = scripts(&directory.path().join("react/src"))
        .iter()
        .map(|path| {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            (
                name.clone(),
                unbound(&fs::read_to_string(path).unwrap(), &name),
            )
        })
        .filter(|(_, names)| !names.is_empty())
        .collect::<Vec<_>>();
    free.sort();
    assert!(free.is_empty(), "unbound references: {free:?}");
}

/// What the verdict above is allowed to conclude from. There are two ways it could pass while
/// proving nothing, so both are pinned here rather than inside it.
///
/// It could read nothing. The point of sweeping the whole project is that `main.jsx` and
/// `runtime/*.mjs` legitimately reach for globals, so they are the evidence that the
/// allow-list is wide enough to be adoptable rather than merely narrow enough to catch one
/// name. Or the branch that carried the defect could simply be gone — deleting it satisfies
/// every binding assertion while dropping the narrow-viewport behaviour it was written for.
#[tokio::test]
async fn checks_a_project_that_still_contains_the_branch_the_defect_lived_in() {
    let directory = tempfile::tempdir().unwrap();
    write_project(&support::specification(), directory.path(), &[])
        .await
        .unwrap();
    let source = directory.path().join("react/src");
    let names = scripts(&source)
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    for expected in ["App.jsx", "main.jsx", "states.jsx", "style.mjs"] {
        assert!(names.contains(&expected.to_string()), "{names:?}");
    }
    let app = fs::read_to_string(source.join("App.jsx")).unwrap();
    assert!(
        app.contains("getAttribute('role')==='tab'&&captured"),
        "the tab-scroll branch is gone, so the binding verdict proves nothing"
    );
    assert!(
        app.contains("<=390&&captured.elements.some"),
        "the narrow-viewport clamp was deleted rather than bound"
    );
}

/// The shadow repair, stated over the artifact. Parsing alone is not the property: dropping
/// the subtree would satisfy it too, and that is the erasure this replaces. So the sentinel
/// must be absent *and* the content it named must be present, nested inside the translation.
#[tokio::test]
async fn translates_the_shadow_sentinel_and_keeps_the_subtree_it_named() {
    let directory = tempfile::tempdir().unwrap();
    write_project(&support::specification(), directory.path(), &[])
        .await
        .unwrap();
    let source = directory.path().join("react/src");
    let sources = scripts(&source)
        .iter()
        .map(|path| {
            (
                path.strip_prefix(&source).unwrap().to_owned(),
                fs::read_to_string(path).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    for (path, text) in &sources {
        assert!(
            !text.contains("#shadow-root"),
            "{path:?} ships the capture sentinel"
        );
    }
    let hosts = sources
        .iter()
        .filter(|(_, text)| text.contains("<ShadowRoot mode={\"open\"}>"))
        .collect::<Vec<_>>();
    assert!(!hosts.is_empty(), "no file opens a shadow root");
    for (path, text) in hosts {
        assert!(
            text.contains("\"Shadowed\""),
            "{path:?} opens a shadow root whose subtree was dropped"
        );
        assert!(
            text.contains("{ShadowRoot}") && text.contains("runtime/shadow.mjs"),
            "{path:?} uses a component it never imports"
        );
    }
}

/// Every emitted module, whichever writer produced it.
fn scripts(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match path.is_dir() {
            true => found.extend(scripts(&path)),
            false => {
                if matches!(
                    path.extension().and_then(|value| value.to_str()),
                    Some("jsx" | "mjs" | "js")
                ) {
                    found.push(path);
                }
            }
        }
    }
    found.sort();
    found
}
