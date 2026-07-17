use crate::cli::InstallArgs;
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn install(args: InstallArgs) -> Result<()> {
    let home = home()?;
    let targets = targets(&args, &home)?;
    let binary = install_binary(&home)?;
    for target in targets {
        let directory = home.join(target).join("skills").join("recreate");
        if directory.is_symlink() {
            bail!("refusing to replace linked skill: {}", directory.display());
        }
        fs::create_dir_all(&directory)?;
        fs::write(directory.join("SKILL.md"), installed_skill(&binary))?;
        println!("installed {}", directory.display());
    }
    Ok(())
}

pub fn workflow() -> &'static str {
    r#"Current Recreate workflow:

1. Get the source URL and destination repository.
2. Open the source in a dedicated visible browser:
   recreate open <url>
   Keep the returned cdp_url and target id. Never use a headless test browser
   for a user access or sign-in step.
3. Inspect that exact rendered target through CDP. Never substitute raw HTML.
4. Decide from the full rendered page whether it is the requested interface or
   an access step. If ambiguous, ask one short question: recreate the visible
   page or wait for the page behind it. Judge from rendered controls and content,
   never URL/title keywords or site-specific login patterns.
5. If access is required, foreground the exact target and ask the user to
   complete access there. Confirm the requested interface is visibly rendered
   in the same target before capture. Keep credentials and session data in-browser.
6. Capture the exact inspected target with an instrumented reload so startup
   motion is observed without navigating to another page:
   recreate capture --reuse --reload --target <id> --cdp-url <cdp_url>
7. Build from react/src/App.jsx and validate acceptance.json.
"#
}

fn installed_skill(binary: &Path) -> String {
    format!(
        r#"---
name: recreate
description: Capture and recreate a live interface with its structure, styling, responsive behavior, assets, and interactions.
license: MIT
---

Run `"{}" skill` first, then follow the printed workflow.
"#,
        binary.display()
    )
}

fn install_binary(home: &Path) -> Result<PathBuf> {
    let source = std::env::current_exe()?;
    let directory = home.join(".recreate").join("bin");
    fs::create_dir_all(&directory)?;
    let name = if cfg!(windows) {
        "recreate.exe"
    } else {
        "recreate"
    };
    let target = directory.join(name);
    if source != target {
        fs::copy(source, &target)?;
    }
    Ok(target)
}

fn targets(args: &InstallArgs, home: &Path) -> Result<Vec<&'static str>> {
    if args.all {
        return Ok(vec![".copilot", ".claude"]);
    }
    let mut values = Vec::new();
    if args.copilot {
        values.push(".copilot");
    }
    if args.claude {
        values.push(".claude");
    }
    if values.is_empty() {
        for name in [".copilot", ".claude"] {
            if home.join(name).exists() {
                values.push(name);
            }
        }
    }
    if values.is_empty() {
        bail!("no Copilot or Claude installation detected");
    }
    Ok(values)
}

fn home() -> Result<PathBuf> {
    std::env::var_os("RECREATE_HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .context("home directory unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_points_to_native_binary() {
        let content = installed_skill(Path::new("/tmp/recreate"));
        assert!(content.contains("recreate"));
        assert!(content.contains("skill"));
        assert!(!content.contains("node "));
        assert!(!content.contains("npm "));
    }

    #[test]
    fn workflow_uses_rendered_access_state_not_login_patterns() {
        assert!(workflow().contains("never URL/title keywords"));
        assert!(workflow().contains("rendered controls and content"));
        assert!(workflow().contains("recreate open <url>"));
        assert!(workflow().contains("instrumented reload"));
        assert!(workflow().contains("Never use a headless test browser"));
    }
}
