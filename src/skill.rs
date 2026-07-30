use crate::{
    cli::{Cli, InstallArgs},
    updater,
};
use anyhow::{Context, Result, bail};
use clap::CommandFactory;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub async fn install(args: InstallArgs) -> Result<()> {
    let home = home()?;
    let targets = targets(&args, &home)?;
    undocumented_commands(installed_skill(), &shipped_commands())
        .map_err(|missing| anyhow::anyhow!("skill documents unavailable commands: {missing}"))?;
    install_binary(&home)?;
    updater::ensure_backtest().await?;
    for target in targets {
        let directory = home.join(target).join("skills").join("recreate");
        if directory.is_symlink() {
            bail!("refusing to replace linked skill: {}", directory.display());
        }
        fs::create_dir_all(&directory)?;
        fs::write(directory.join("SKILL.md"), installed_skill())?;
        println!("installed {}", directory.display());
    }
    Ok(())
}

fn installed_skill() -> &'static str {
    r#"---
name: recreate
description: Recreate a web page in React and compare it to the source.
license: MIT
---

Recreate is a tool for capturing a given source web page, generating a recreation that is written in React, and providing a text based diff of the two.

1. Run `recreate open <source-url>` and request browser access only if needed.
2. Run `recreate capture --reuse --reload` to capture that source and generate its React recreation.
3. Make the recreation available at a URL, such as `http://localhost:8080`.
4. Run `recreate backtest run --source <source-url> --recreation <recreation-url>`.
5. Use the plain-English findings to fix content, layout, typography, interactions, and motion; repeat until clean.
6. To inspect one part, add `--focus "<name>"`, for example `--focus "toolbar"` or `--focus "App launcher"`; still run the full comparison before finishing.
7. Verify every visible interaction and authored motion.
8. Finish only when the full comparison is conclusive, under five seconds, and has no unresolved or duplicate findings.
9. Present the result as debugging evidence, not fidelity certification.

Parameters used:

- `<source-url>`: the original page to compare.
- `<recreation-url>`: the running recreation to compare, including localhost.
- `--reuse`: use the source page opened in step 1.
- `--reload`: include the page's startup behavior.
- `--source`: set the source URL for the comparison.
- `--recreation`: set the recreation URL for the comparison.
- `--focus`: case-insensitive name search, not a CSS selector. It matches visible text, accessible names, and semantic regions such as toolbar, navigation, banner, main, or dialog on both pages. Multiple matches are included; if either page has no match, the command reports that selection failure.

Use `recreate <command> --help` for optional advanced parameters.
"#
}

fn shipped_commands() -> BTreeSet<String> {
    Cli::command()
        .get_subcommands()
        .map(|command| command.get_name().to_owned())
        .collect()
}

/// The skill is the agent's only contract, so a build must never install
/// instructions naming a command it does not expose.
fn undocumented_commands(
    skill: &str,
    shipped: &BTreeSet<String>,
) -> std::result::Result<(), String> {
    let mut missing = BTreeSet::new();
    for (index, _) in skill.match_indices("recreate ") {
        let rest = &skill[index + "recreate ".len()..];
        let name: String = rest
            .chars()
            .take_while(|character| character.is_ascii_lowercase())
            .collect();
        if name.is_empty() {
            continue;
        }
        if !shipped.contains(&name) {
            missing.insert(name);
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(missing.into_iter().collect::<Vec<_>>().join(", "))
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
    fn installed_skill_only_documents_shipped_commands() {
        undocumented_commands(installed_skill(), &shipped_commands())
            .expect("installed skill must not document commands the binary lacks");
    }

    #[test]
    fn a_documented_command_the_binary_lacks_is_rejected() {
        let shipped = ["capture".to_owned()].into_iter().collect();
        let error = undocumented_commands("Run `recreate backtest run --source x`.", &shipped)
            .expect_err("a missing command must fail the install");
        assert_eq!(error, "backtest");
    }

    #[test]
    fn placeholders_and_prose_are_not_treated_as_commands() {
        let shipped = ["capture".to_owned()].into_iter().collect();
        undocumented_commands(
            "Recreate is a tool. Use `recreate <command> --help` and `recreate capture`.",
            &shipped,
        )
        .expect("placeholders and prose must not be mistaken for commands");
    }

    #[test]
    fn installed_skill_exposes_the_complete_workflow() {
        let content = installed_skill();
        assert_eq!(content.matches("\n1. ").count(), 1);
        assert!(content.contains("\n9. "));
        assert!(!content.contains("\n10. "));
    }

    #[test]
    fn installed_skill_contains_backtest_gates() {
        let content = installed_skill();
        assert!(content.contains("--source <source-url>"));
        assert!(content.contains("--recreation <recreation-url>"));
        assert!(content.contains("--focus \"<name>\""));
        assert!(content.contains("including localhost"));
        assert!(content.contains("not a CSS selector"));
        assert!(content.contains("Multiple matches are included"));
        assert!(content.contains("selection failure"));
        assert!(content.contains("plain-English findings"));
        assert!(content.contains("recreate capture --reuse --reload"));
        assert!(content.contains("Parameters used:"));
        assert!(content.contains("use the source page opened in step 1"));
        assert!(content.contains("include the page's startup behavior"));
        assert!(content.contains("under five seconds"));
        assert!(content.contains("debugging evidence"));
        assert!(!content.contains("ask the user to run"));
        assert!(!content.contains("dependencies"));
        assert!(!content.contains("No arguments"));
        assert!(!content.contains("recreate-backtest"));
        assert!(!content.contains(".recreate"));
        assert!(!content.contains("--target"));
        assert!(!content.contains("--cdp-url"));
        assert!(!content.contains("Locate"));
        assert!(!content.contains("cargo build"));
        assert!(content.contains("not fidelity certification"));
    }
}
