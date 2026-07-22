use std::path::{Path, PathBuf};

pub fn find() -> Option<PathBuf> {
    candidates().into_iter().find(|path| path.exists())
}

fn candidates() -> Vec<PathBuf> {
    if cfg!(windows) {
        let roots = [
            std::env::var_os("PROGRAMFILES(X86)"),
            std::env::var_os("PROGRAMFILES"),
            std::env::var_os("LOCALAPPDATA"),
        ];
        return roots
            .into_iter()
            .flatten()
            .flat_map(|root| {
                let root = Path::new(&root);
                [
                    root.join("Microsoft/Edge/Application/msedge.exe"),
                    root.join("Google/Chrome/Application/chrome.exe"),
                ]
            })
            .collect();
    }
    vec![
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
        "/usr/bin/google-chrome".into(),
        "/usr/bin/chromium".into(),
        "/usr/bin/microsoft-edge".into(),
    ]
}
