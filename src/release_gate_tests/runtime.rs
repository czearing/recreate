use anyhow::{Context, Result, bail};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    thread::{self, JoinHandle},
};

pub struct Server {
    address: String,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    pub fn start(root: &Path) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?.to_string();
        let root = root.to_path_buf();
        let thread = thread::spawn(move || serve(listener, &root));
        Ok(Self {
            address,
            thread: Some(thread),
        })
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = TcpStream::connect(&self.address)
            .and_then(|mut stream| stream.write_all(b"GET /__shutdown HTTP/1.1\r\n\r\n"));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn build(root: &Path) -> Result<()> {
    run_npm(
        root,
        &[
            "install",
            "--save-exact",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "vite@8.1.2",
            "react@19.2.0",
            "react-dom@19.2.0",
        ],
    )?;
    run_npm(root, &["run", "build"])?;
    if !root.join("dist/index.html").exists() {
        bail!("generated Vite build did not create dist/index.html");
    }
    Ok(())
}

fn run_npm(root: &Path, args: &[&str]) -> Result<()> {
    let executable = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let status = Command::new(executable)
        .args(args)
        .current_dir(root)
        .env("CI", "1")
        .status()
        .with_context(|| format!("run npm {}", args.join(" ")))?;
    if !status.success() {
        bail!("npm {} failed with {status}", args.join(" "));
    }
    Ok(())
}

fn serve(listener: TcpListener, root: &Path) {
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { break };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(1)));
        let Some(path) = request_path(&mut stream) else {
            continue;
        };
        if path == "/__shutdown" {
            break;
        }
        let path = asset_path(root, &path);
        let (status, body) = match fs::read(&path) {
            Ok(body) => ("200 OK", body),
            Err(_) => ("404 Not Found", b"not found".to_vec()),
        };
        let content_type = content_type(&path);
        let header = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(&body);
    }
}

fn request_path(stream: &mut TcpStream) -> Option<String> {
    let mut request = [0; 4096];
    let size = stream.read(&mut request).ok()?;
    let text = std::str::from_utf8(&request[..size]).ok()?;
    text.lines()
        .next()?
        .split_whitespace()
        .nth(1)
        .map(str::to_string)
}

fn asset_path(root: &Path, request: &str) -> PathBuf {
    let relative = request.split('?').next().unwrap_or("/");
    if relative == "/" {
        return root.join("index.html");
    }
    let safe = relative
        .trim_start_matches('/')
        .split('/')
        .filter(|part| !part.is_empty() && *part != "..");
    root.join(safe.collect::<PathBuf>())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("css") => "text/css",
        Some("js") => "text/javascript",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "text/html; charset=utf-8",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_paths_cannot_escape_build_root() {
        let root = Path::new("dist");
        assert_eq!(asset_path(root, "/../secret"), root.join("secret"));
        assert_eq!(asset_path(root, "/"), root.join("index.html"));
    }
}
