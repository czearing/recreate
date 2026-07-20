use anyhow::Result;
use std::{fs, path::Path};

const MODULES: [(&str, &str); 5] = [
    (
        "interaction.mjs",
        include_str!("../../runtime/interaction.mjs"),
    ),
    ("sequence.mjs", include_str!("../../runtime/sequence.mjs")),
    ("carousel.mjs", include_str!("../../runtime/carousel.mjs")),
    ("textarea.mjs", include_str!("../../runtime/textarea.mjs")),
    ("anchor.mjs", include_str!("../../runtime/anchor.mjs")),
];

pub fn write(source: &Path) -> Result<()> {
    let runtime = source.join("runtime");
    fs::create_dir_all(&runtime)?;
    for (name, contents) in MODULES {
        fs::write(runtime.join(name), contents)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn embeds_the_exact_tested_runtime_modules() {
        for (_, contents) in super::MODULES {
            assert!(!contents.trim().is_empty());
            assert!(contents.contains("export "));
        }
    }
}
