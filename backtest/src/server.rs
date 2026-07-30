use axum::{
    Router,
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode, Uri, header},
    response::Response,
};
use std::{
    path::{Component, Path, PathBuf},
    sync::Arc,
};
use tokio::{net::TcpListener, task::JoinHandle};

pub struct Server {
    pub base_url: String,
    task: JoinHandle<()>,
}

impl Server {
    pub async fn start(root: PathBuf) -> anyhow::Result<Self> {
        let root = root.canonicalize()?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let app = Router::new().fallback(serve).with_state(Arc::new(root));
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(Self {
            base_url: format!("http://{address}"),
            task,
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn serve(State(root): State<Arc<PathBuf>>, uri: Uri) -> Response {
    match safe_path(&root, uri.path()) {
        Some(path) => match tokio::fs::read(&path).await {
            Ok(bytes) => {
                let mime = match path.extension().and_then(|value| value.to_str()) {
                    Some("html") => "text/html; charset=utf-8",
                    Some("css") => "text/css; charset=utf-8",
                    Some("js") => "text/javascript; charset=utf-8",
                    Some("json") => "application/json",
                    Some("png") => "image/png",
                    _ => "application/octet-stream",
                };
                let mut response = Response::new(Body::from(bytes));
                response
                    .headers_mut()
                    .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            }
            Err(_) => response(StatusCode::NOT_FOUND, "not found"),
        },
        None => response(StatusCode::BAD_REQUEST, "invalid path"),
    }
}

fn safe_path(root: &Path, requested: &str) -> Option<PathBuf> {
    let relative = requested.trim_start_matches('/');
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(value) => path.push(value),
            _ => return None,
        }
    }
    if path.is_dir() {
        path.push("index.html");
    }
    Some(path)
}

fn response(status: StatusCode, text: &'static str) -> Response {
    Response::builder()
        .status(status)
        .body(Body::from(text))
        .expect("valid response")
}
