/// Fails before the capture fix: the walk read only `document.styleSheets` and
/// discarded every sheet whose `cssRules` threw, so a page whose CSS is served from
/// a CDN, adopted as a constructed sheet, or scoped to a shadow root produced no
/// authored rules and every element was rebuilt from sampled pixels.
#[test]
fn the_capture_script_reads_sheets_the_page_cannot_read_itself() {
    let source = crate::page_script::source_with_sheets(&[]);
    assert!(source.contains("authoredSheetTexts"));
    assert!(source.contains("adoptedStyleSheets"));
    assert!(source.contains("collectShadowSheets"));
    assert!(!source.contains("try { visitRules(sheet.cssRules); } catch {}"));
}

/// The supplied text has to arrive as data inside the script. Wrapping the script in
/// another function to assign a global changes what the expression evaluates to — the
/// capture returns a promise, and the wrapper made it return the wrong thing with no
/// error, which every unit test still passed.
#[test]
fn supplied_stylesheet_text_is_injected_without_rewrapping_the_script() {
    let source = crate::page_script::source_with_sheets(&[".a{color:red}".into()]);
    assert!(source.contains(".a{color:red}"));
    assert!(source.trim_start().starts_with("(async () => {"));
    assert!(source.trim_end().ends_with("})()"));
    assert!(!source.contains("__AUTHORED_SHEETS__"));
}
