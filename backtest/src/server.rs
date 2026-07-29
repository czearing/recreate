use anyhow::Context;
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
};

pub struct Server {
    pub base_url: String,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl Server {
    pub async fn start(root: PathBuf) -> anyhow::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        let root = Arc::new(root);
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown_rx => break,
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break };
                        let root = root.clone();
                        tokio::spawn(async move {
                            let _ = handle(stream, &root).await;
                        });
                    }
                }
            }
        });
        Ok(Self {
            base_url: format!("http://{address}"),
            shutdown: Some(shutdown_tx),
            task,
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.abort();
    }
}

async fn handle(mut stream: TcpStream, root: &Path) -> anyhow::Result<()> {
    let mut buffer = vec![0; 16 * 1024];
    let count = stream.read(&mut buffer).await?;
    if count == 0 {
        return Ok(());
    }
    let request = String::from_utf8_lossy(&buffer[..count]);
    let line = request.lines().next().unwrap_or_default();
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let requested = parts.next().unwrap_or("/");
    if !matches!(method, "GET" | "HEAD") {
        return respond(&mut stream, 405, "text/plain", b"method not allowed", method == "HEAD")
            .await;
    }
    let path = safe_path(root, requested)?;
    let path = if path.is_dir() {
        path.join("index.html")
    } else {
        path
    };
    if !path.exists() {
        return respond(&mut stream, 404, "application/json", br#"{"ok":false}"#, method == "HEAD")
            .await;
    }
    let bytes = tokio::fs::read(&path)
        .await
        .with_context(|| format!("read {}", path.display()))?;
    respond(&mut stream, 200, mime(&path), &bytes, method == "HEAD").await
}

fn safe_path(root: &Path, requested: &str) -> anyhow::Result<PathBuf> {
    let path = requested
        .split('?')
        .next()
        .unwrap_or("/")
        .trim_start_matches('/');
    let mut result = root.to_path_buf();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            _ => anyhow::bail!("invalid request path"),
        }
    }
    Ok(result)
}

async fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head: bool,
) -> anyhow::Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    if !head {
        stream.write_all(body).await?;
    }
    stream.shutdown().await?;
    Ok(())
}

fn mime(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_segments() {
        assert!(safe_path(Path::new("fixtures"), "/../secret").is_err());
    }
}

