//! The single owner of "when is `assetData` complete?".
//!
//! Two capture scripts define that table — one that fetches every referenced subresource,
//! and one that does not because the state it records is a delta on a page already
//! captured. Both must end holding the content read directly off the elements that carry
//! no reference to it, so the two endings are rendered here rather than written at each
//! injection site, where only one of them would have been remembered.

/// Completes the table with the content no reference names.
const SURFACES: &str = "\n  Object.assign(assetData, recreateSurfaceAssets());\n";

pub(crate) const DOWNLOADS: &str = r#"
  const assetData = {};
  await Promise.all(Array.from(assets)
    .filter(url => !url.startsWith('data:'))
    .map(async url => {
      try {
        const response = await fetch(url, {credentials: 'include'});
        const type = response.headers.get('content-type') || '';
        if (!response.ok || type.includes('text/html')) return;
        const blob = await response.blob();
        assetData[url] = await new Promise((resolve, reject) => {
          const reader = new FileReader();
          reader.onload = () => resolve(reader.result);
          reader.onerror = reject;
          reader.readAsDataURL(blob);
        });
      } catch {}
    }));
"#;

/// Every referenced subresource, then the unreferenced content.
pub fn with_downloads() -> String {
    format!("{DOWNLOADS}{SURFACES}")
}

/// The unreferenced content alone, for a capture whose subresources are already held.
pub fn without_downloads() -> String {
    format!("  const assetData = {{}};{SURFACES}")
}
