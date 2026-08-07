use anyhow::{Context, Result};
use std::{
    net::SocketAddr,
    path::{Component, Path, PathBuf},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

/// A local directory served over real HTTP.
///
/// Capture must behave identically for a local directory and a remote site.
/// A `file://` page is an opaque origin, so scripts, module imports, and
/// `cssRules` on its own stylesheets fail there but succeed over HTTP. Serving
/// the directory keeps one code path for both inputs.
pub struct Directory {
    pub url: String,
    task: JoinHandle<()>,
}

impl Directory {
    pub async fn serve(root: &Path) -> Result<Self> {
        let root = root
            .canonicalize()
            .with_context(|| format!("directory not found: {}", root.display()))?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .context("bind local capture server")?;
        let address: SocketAddr = listener.local_addr()?;
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let root = root.clone();
                tokio::spawn(async move {
                    let _ = respond(stream, &root).await;
                });
            }
        });
        Ok(Self {
            url: format!("http://localhost:{}", address.port()),
            task,
        })
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Treats a positional capture argument as a directory when it names one on disk.
pub fn as_directory(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    path.is_dir().then(|| path.to_path_buf())
}

async fn respond(mut stream: TcpStream, root: &Path) -> Result<()> {
    let mut request = vec![0u8; 8192];
    let read = stream.read(&mut request).await?;
    let head = String::from_utf8_lossy(&request[..read]);
    let target = head
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .split(['?', '#'])
        .next()
        .unwrap_or("/");
    match resolve(root, target).and_then(|path| std::fs::read(&path).ok().map(|body| (path, body)))
    {
        Some((path, body)) => {
            let mime = content_type(&path);
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).await?;
            stream.write_all(&body).await?;
        }
        None => {
            stream
                .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await?;
        }
    }
    stream.flush().await?;
    Ok(())
}

/// Rejects any traversal component so a served scene cannot read outside its root.
fn resolve(root: &Path, target: &str) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for component in Path::new(target.trim_start_matches('/')).components() {
        match component {
            Component::Normal(value) => path.push(value),
            Component::CurDir => {}
            _ => return None,
        }
    }
    if path.is_dir() {
        path.push("index.html");
    }
    path.is_file().then_some(path)
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_directory_argument_is_recognized_and_a_url_is_not() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            as_directory(directory.path().to_str().unwrap()),
            Some(directory.path().to_path_buf())
        );
        assert_eq!(as_directory("https://example.com"), None);
    }

    #[test]
    fn a_directory_request_resolves_to_its_index() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("index.html"), "<p>scene</p>").unwrap();
        assert_eq!(
            resolve(directory.path(), "/"),
            Some(directory.path().join("index.html"))
        );
    }

    #[test]
    fn traversal_outside_the_root_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("index.html"), "<p>scene</p>").unwrap();
        assert_eq!(resolve(directory.path(), "/../secret.txt"), None);
    }

    #[tokio::test]
    async fn a_served_scene_is_reachable_over_http() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("index.html"), "<p>scene</p>").unwrap();
        let served = Directory::serve(directory.path()).await.unwrap();
        let response = reqwest::get(&served.url).await.unwrap();
        assert!(response.status().is_success());
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        assert_eq!(response.text().await.unwrap(), "<p>scene</p>");
    }
}
