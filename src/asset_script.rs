pub const SOURCE: &str = r#"
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
