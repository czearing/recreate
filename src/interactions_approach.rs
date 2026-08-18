//! How a pass reaches the element it is about to touch.
//!
//! Every interaction begins by bringing a control within reach, and no pass may invent its own
//! way of doing that: the scroll it takes to get there is the harness moving rather than the
//! page reacting, and a pass that scrolled on its own would leave that movement recorded as
//! the action's own effect.

/// Bringing an element within reach, defined once for every pass that has to touch one.
///
/// The approach records whether it had to move — the generated app reads
/// `data-recreate-preserve-scroll` to decide whether to replay a captured offset — and tells
/// the action scope where it left every scroller it touched. A scroll event is delivered
/// asynchronously, so the harness's own arrives long after the approach returned; what
/// identifies it is that the scroller is still exactly where the approach left it.
const APPROACH: &str = "const before=[scrollX,scrollY]; const ancestors=[]; \
     for(let node=element.parentElement;node&&node!==document.documentElement;node=node.parentElement) \
       ancestors.push([node,node.scrollLeft,node.scrollTop]); \
     element.scrollIntoView({block:'center',inline:'center',behavior:'instant'}); \
     element.dataset.recreatePreserveScroll=String( \
       scrollX!==before[0]||scrollY!==before[1]|| \
       ancestors.some(([node,left,top])=>node.scrollLeft!==left||node.scrollTop!==top)); \
     globalThis.__recreateCaptureScope?.settle( \
       [document.scrollingElement,...ancestors.map(([node])=>node)]);";

/// The approach as every caller receives it, so no pass can position an element on its own.
#[cfg(test)]
pub fn approach_script() -> &'static str {
    APPROACH
}

/// Pointing at a control: position, then hover, then focus on the next frame.
///
/// Focus is a consequence of pointing, not a behaviour of the page, so it has to land before
/// the evidence for an activation is measured. Measuring across the focus change instead makes
/// every focusable control look like it did something, because a focus ring is a style change.
pub fn aim(path: &str, matching: &str, fallback: &str) -> String {
    format!(
        "(() => {{ const matches={matching};const exact=document.querySelector({path});\
         const element=matches(exact)?exact:({fallback});if(!element)return false; \
         {APPROACH} \
         element.dispatchEvent(new PointerEvent('pointerover',{{bubbles:true}})); \
         element.dispatchEvent(new MouseEvent('mouseover',{{bubbles:true}})); \
         return new Promise(resolve=>requestAnimationFrame(()=>{{ \
           element.focus({{preventScroll:true}}); \
           const rect=element.getBoundingClientRect(); \
           resolve([rect.x+rect.width/2,rect.y+rect.height/2]); \
         }})); }})()"
    )
}

/// Filling a control: position, focus, then set the value the way the page's own listeners
/// see it, which is through the prototype setter rather than the property.
pub fn text_entry(path: &str, tag: &str, label: &str, fallback: &str) -> String {
    format!(
        r#"(() => {{
          const labelOf=candidate=>(candidate.getAttribute('aria-label')||
            candidate.getAttribute('placeholder')||candidate.innerText||
            candidate.value||'').replace(/\s+/g,' ').trim();
          const matches=candidate=>candidate&&
            candidate.tagName.toLowerCase()==={tag}&&labelOf(candidate)==={label};
          const exact=document.querySelector({path});
          const element=matches(exact)?exact:({fallback});
          if(!element)return false;
          {APPROACH}
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
        }})()"#
    )
}

/// Every pass that has to bring an element within reach, rendered as it ships.
#[cfg(test)]
pub fn positioning_scripts() -> Vec<(&'static str, String)> {
    let path = "\"html>body:nth-of-type(1)>button:nth-of-type(1)\"";
    vec![
        ("aim", aim(path, "candidate=>candidate", "null")),
        (
            "text entry",
            text_entry(path, "\"input\"", "\"Name\"", "null"),
        ),
    ]
}
