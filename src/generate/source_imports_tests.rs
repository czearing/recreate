//! What each emitted file is allowed to import, decided over the finished tree.

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
    let source =
        "import {createPortal} from 'react-dom';\nconst view=createPortal(null,document.body);\n";
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
    let used = "import {moveCarousel as move} from './carousel.mjs';\nconst next=move(state);\n";
    assert_eq!(super::prune(used), None);
    let unused =
        "import {moveCarousel as move} from './carousel.mjs';\nconst next=moveCarousel(state);\n";
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

/// A binding a writer emits without arranging for it: the import is supplied, and its path is
/// spelt from where the file actually sits, which is the value no writer can know.
#[test]
fn supplies_a_runtime_binding_the_file_uses_but_never_bound() {
    let root = std::path::Path::new("/p/src");
    let source = "export default function View(){return <ShadowRoot mode={\"open\"}/>}\n";
    assert_eq!(
        super::supply(source, root, &root.join("states/Interaction0/Frame.jsx")).unwrap(),
        format!("import {{ShadowRoot}} from '../../runtime/shadow.mjs';\n{source}")
    );
    assert_eq!(
        super::supply(source, root, &root.join("App.jsx")).unwrap(),
        format!("import {{ShadowRoot}} from './runtime/shadow.mjs';\n{source}")
    );
}

/// Supplying is a narrowing too. A file that never mentions the binding is left alone, or
/// every emitted module would grow an import that the pruner then has to take away again.
#[test]
fn leaves_a_file_that_never_mentions_the_binding_alone() {
    let root = std::path::Path::new("/p/src");
    let source = "export default function View(){return <div/>}\n";
    assert_eq!(super::supply(source, root, &root.join("views/A.jsx")), None);
    let bound = "import {ShadowRoot} from '../runtime/shadow.mjs';\nconst a=<ShadowRoot/>;\n";
    assert_eq!(super::supply(bound, root, &root.join("views/A.jsx")), None);
}

/// The module that defines the binding must never be given an import of itself, which is the
/// one file where the mention is a declaration rather than a use.
#[test]
fn never_makes_the_providing_module_import_itself() {
    let root = std::path::Path::new("/p/src");
    let source = "export function ShadowRoot({children}){return children}\n";
    assert_eq!(
        super::supply(source, root, &root.join("runtime/shadow.mjs")),
        None
    );
}
