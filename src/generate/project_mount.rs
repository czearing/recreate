use anyhow::Result;

pub(super) fn mount(has_root: bool, root_class: &str) -> Result<(String, &'static str)> {
    if !has_root {
        return Ok(("createRoot(document.body).render(<App />);".into(), ""));
    }
    Ok((
        format!(
            "const root=document.getElementById('root');\nroot.className={};\ncreateRoot(root).render(<App />);",
            serde_json::to_string(root_class)?
        ),
        "<div id=\"root\"></div>",
    ))
}
