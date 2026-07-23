use crate::browser::Browser;

const CONTROL_STATE: &str = r#"(() => JSON.stringify({
  dom:[...document.body.querySelectorAll('*')].map(e=>[
    e.localName,e.getAttribute('class')||'',e.getAttribute('style')||'',
    e.hasAttribute('hidden'),e.src?.startsWith('blob:')?'blob:':(e.getAttribute('src')||''),
    e.scrollLeft,e.scrollTop
  ]),
  controls:[...document.querySelectorAll(
    'a[href],button,input,select,textarea,summary,[role],[tabindex],[contenteditable="true"]'
  )].map(e=>({
    tag:e.localName,
    attrs:['aria-expanded','aria-selected','aria-pressed','aria-checked','aria-disabled']
      .map(name=>[name,e.getAttribute(name)]),
    value:e.value??null,checked:e.checked??null,
    disabled:e.disabled??null
  })),
  document:[scrollX,scrollY]
}))()"#;

pub(crate) async fn state(browser: &mut Browser) -> anyhow::Result<String> {
    browser
        .cdp
        .evaluate(CONTROL_STATE)
        .await?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("browser control state is unavailable"))
}
