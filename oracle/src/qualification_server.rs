use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

pub struct Server {
    address: SocketAddr,
    pages: Arc<Mutex<BTreeMap<usize, String>>>,
    stopping: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    pub fn start() -> anyhow::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let pages = Arc::new(Mutex::new(BTreeMap::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stopping);
        let served_pages = Arc::clone(&pages);
        let thread = thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                respond(stream, &served_pages);
            }
        });
        Ok(Self {
            address,
            pages,
            stopping,
            thread: Some(thread),
        })
    }

    pub fn url(&self, value: bool) -> String {
        format!("http://{}/payload/{value}", self.address)
    }

    pub fn page(&self, html: String) -> String {
        let mut pages = self.pages.lock().expect("qualification pages poisoned");
        let id = pages.len();
        pages.insert(id, html);
        format!("http://{}/page/{id}", self.address)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn respond(mut stream: TcpStream, pages: &Mutex<BTreeMap<usize, String>>) {
    let mut request = [0; 1024];
    let read = stream.read(&mut request).unwrap_or(0);
    let request = String::from_utf8_lossy(&request[..read]);
    let path = request.split_whitespace().nth(1).unwrap_or("/");
    let (mime, body) = if path == "/payload/false" {
        ("application/json", r#"{"ok":false}"#.into())
    } else if path == "/payload/true" {
        ("application/json", r#"{"ok":true}"#.into())
    } else if let Some(id) = path.strip_prefix("/page/").and_then(|id| id.parse().ok()) {
        (
            "text/html; charset=utf-8",
            pages
                .lock()
                .expect("qualification pages poisoned")
                .get(&id)
                .cloned()
                .unwrap_or_default(),
        )
    } else {
        ("text/plain", String::new())
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}
