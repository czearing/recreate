pub async fn count(endpoint: &str) -> anyhow::Result<usize> {
    Ok(recreate_browser::list(endpoint).await?.len())
}
