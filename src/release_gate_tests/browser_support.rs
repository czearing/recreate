use std::{
    path::{Path, PathBuf},
    process::{Child, Command},
    time::Duration,
};

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub fn launch_browser(path: &Path, profile: &Path, port: u16) -> Child {
    Command::new(path)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-first-run",
            "--no-default-browser-check",
            &format!("--remote-debugging-port={port}"),
            &format!("--user-data-dir={}", profile.display()),
            "about:blank",
        ])
        .spawn()
        .unwrap()
}

pub async fn wait_for_browser(port: u16) {
    let url = format!("http://127.0.0.1:{port}/json/version");
    for _ in 0..300 {
        if reqwest::get(&url)
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("browser did not start");
}

pub fn browser_path() -> Option<PathBuf> {
    std::env::var_os("RECREATE_BROWSER")
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| {
            [
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                "/usr/bin/google-chrome",
                "/usr/bin/chromium",
                "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            ]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| path.exists())
        })
}
