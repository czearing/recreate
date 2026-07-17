use crate::cdp::Cdp;
use anyhow::Result;

pub async fn focused_path(cdp: &mut Cdp) -> Result<Option<String>> {
    let value = cdp
        .evaluate(
            r#"(() => {
              const element = document.activeElement;
              if (!element || element === document.body) return null;
              const parts = [];
              for (let node = element; node && node !== document.documentElement; node = node.parentElement) {
                const peers = node.parentElement
                  ? Array.from(node.parentElement.children).filter(child => child.tagName === node.tagName)
                  : [node];
                parts.push(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node) + 1})`);
              }
              return `html>${parts.reverse().join('>')}`;
            })()"#,
        )
        .await?;
    Ok(value.as_str().map(str::to_string))
}

pub async fn click(cdp: &mut Cdp, path: &str) -> Result<bool> {
    let expression = format!(
        "(() => {{ const element=document.querySelector({}); if(!element)return false; \
         const before=[scrollX,scrollY]; const ancestors=[]; \
         for(let node=element.parentElement;node&&node!==document.documentElement;node=node.parentElement) \
           ancestors.push([node,node.scrollLeft,node.scrollTop]); \
         element.scrollIntoView({{block:'center',inline:'center'}}); \
         element.dataset.recreatePreserveScroll=String( \
           scrollX!==before[0]||scrollY!==before[1]|| \
           ancestors.some(([node,left,top])=>node.scrollLeft!==left||node.scrollTop!==top)); \
         element.focus({{preventScroll:true}}); element.click(); \
         delete element.dataset.recreatePreserveScroll; return true; }})()",
        serde_json::to_string(path)?
    );
    Ok(cdp.evaluate(&expression).await?.as_bool() == Some(true))
}
