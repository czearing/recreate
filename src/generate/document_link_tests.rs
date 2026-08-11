use super::{resolvable_link, safe_link};

#[test]
fn excludes_executable_source_preloads() {
    assert!(!safe_link(Some("modulepreload"), None));
    assert!(!safe_link(Some("preload"), Some("script")));
    assert!(safe_link(Some("stylesheet"), None));
    assert!(safe_link(Some("icon"), None));
}

/// A relative href names the source site's build output, so the recreation
/// would request a file it never generates and the browser logs a 404.
#[test]
fn excludes_links_to_the_source_projects_own_files() {
    assert!(!resolvable_link(Some("./assets/index-BIZnfT4P.css")));
    assert!(!resolvable_link(Some("./onenote-favicon.svg")));
    assert!(!resolvable_link(Some("/static/app.css")));
    assert!(!resolvable_link(None));
}

#[test]
fn keeps_links_that_still_resolve() {
    assert!(resolvable_link(Some("https://fonts.example.com/font.css")));
    assert!(resolvable_link(Some("http://cdn.example.com/a.css")));
    assert!(resolvable_link(Some("//cdn.example.com/a.css")));
    assert!(resolvable_link(Some("data:,")));
}
