use crate::{browser::Browser, cli::BrowserArgs};

pub async fn start(args: &BrowserArgs) -> anyhow::Result<Browser> {
    if let Some(endpoint) = &args.cdp_url {
        Browser::attach(endpoint.clone(), args.target.as_deref()).await
    } else {
        Browser::launch(args.browser.clone()).await
    }
}
