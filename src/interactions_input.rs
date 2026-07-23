use crate::cdp::Cdp;
use anyhow::Result;

pub fn text_entry(tag: &str) -> bool {
    tag.eq_ignore_ascii_case("textarea") || tag.eq_ignore_ascii_case("input")
}

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

pub async fn click_matching(
    cdp: &mut Cdp,
    path: &str,
    tag: &str,
    label: &str,
    occurrence: Option<usize>,
    require_control: bool,
) -> Result<bool> {
    let (matching, fallback) = if tag.is_empty() {
        ("candidate=>candidate".into(), "null".into())
    } else {
        let tag = serde_json::to_string(tag)?;
        let label = serde_json::to_string(label)?;
        let control = if require_control {
            "candidate.hasAttribute('data-recreate-control')&&"
        } else {
            ""
        };
        let fallback = occurrence.map_or_else(
            || format!("Array.from(document.querySelectorAll({tag})).find(matches)"),
            |index| {
                format!("Array.from(document.querySelectorAll({tag})).filter(matches)[{index}]")
            },
        );
        (
            format!(
                "candidate=>candidate&&{control}candidate.tagName.toLowerCase()==={tag}&&\
                 (candidate.getAttribute('aria-label')||candidate.innerText||candidate.value||'')\
                 .replace(/\\s+/g,' ').trim()==={label}"
            ),
            fallback,
        )
    };
    let expression = format!(
        "(() => {{ const matches={matching};const exact=document.querySelector({});\
         const element=matches(exact)?exact:({fallback});if(!element)return false; \
         const before=[scrollX,scrollY]; const ancestors=[]; \
         for(let node=element.parentElement;node&&node!==document.documentElement;node=node.parentElement) \
           ancestors.push([node,node.scrollLeft,node.scrollTop]); \
         element.scrollIntoView({{block:'center',inline:'center',behavior:'instant'}}); \
         element.dataset.recreatePreserveScroll=String( \
           scrollX!==before[0]||scrollY!==before[1]|| \
           ancestors.some(([node,left,top])=>node.scrollLeft!==left||node.scrollTop!==top)); \
         element.dispatchEvent(new PointerEvent('pointerover',{{bubbles:true}})); \
         element.dispatchEvent(new MouseEvent('mouseover',{{bubbles:true}})); \
         return new Promise(resolve=>requestAnimationFrame(()=>{{ \
           element.focus({{preventScroll:true}}); \
           const rect=element.getBoundingClientRect(); \
           resolve([rect.x+rect.width/2,rect.y+rect.height/2]); \
         }})); }})()",
        serde_json::to_string(path)?
    );
    let position = cdp.evaluate(&expression).await?;
    let Some(position) = position.as_array() else {
        return Ok(false);
    };
    let (Some(x), Some(y)) = (
        position.first().and_then(serde_json::Value::as_f64),
        position.get(1).and_then(serde_json::Value::as_f64),
    ) else {
        return Ok(false);
    };
    for event_type in ["mouseMoved", "mousePressed", "mouseReleased"] {
        let mut params = serde_json::json!({"type":event_type,"x":x,"y":y});
        if event_type != "mouseMoved" {
            params["button"] = serde_json::json!("left");
            params["clickCount"] = serde_json::json!(1);
        }
        cdp.send("Input.dispatchMouseEvent", params).await?;
    }
    cdp.evaluate(&format!(
        "document.querySelector({})?.removeAttribute('data-recreate-preserve-scroll')",
        serde_json::to_string(path)?
    ))
    .await?;
    Ok(true)
}

pub async fn submit_text_matching(
    cdp: &mut Cdp,
    path: &str,
    tag: &str,
    label: &str,
    occurrence: Option<usize>,
) -> Result<bool> {
    let tag_json = serde_json::to_string(tag)?;
    let label_json = serde_json::to_string(label)?;
    let fallback = occurrence.map_or_else(
        || format!("Array.from(document.querySelectorAll({tag_json})).find(matches)"),
        |index| {
            format!("Array.from(document.querySelectorAll({tag_json})).filter(matches)[{index}]")
        },
    );
    let expression = format!(
        r#"(() => {{
          const labelOf=candidate=>(candidate.getAttribute('aria-label')||
            candidate.getAttribute('placeholder')||candidate.innerText||
            candidate.value||'').replace(/\s+/g,' ').trim();
          const matches=candidate=>candidate&&
            candidate.tagName.toLowerCase()==={tag_json}&&labelOf(candidate)==={label_json};
          const exact=document.querySelector({});
          const element=matches(exact)?exact:({fallback});
          if(!element)return false;
          element.scrollIntoView({{block:'center',inline:'center',behavior:'instant'}});
          element.focus({{preventScroll:true}});
          const value='recreate probe';
          const prototype=element instanceof HTMLTextAreaElement
            ? HTMLTextAreaElement.prototype:HTMLInputElement.prototype;
          Object.getOwnPropertyDescriptor(prototype,'value').set.call(element,value);
          element.dispatchEvent(new InputEvent('input',{{
            bubbles:true,inputType:'insertText',data:value
          }}));
          element.dispatchEvent(new Event('change',{{bubbles:true}}));
          return true;
        }})()"#,
        serde_json::to_string(path)?
    );
    Ok(cdp.evaluate(&expression).await?.as_bool() == Some(true))
}
