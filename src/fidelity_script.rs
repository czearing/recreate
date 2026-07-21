pub fn prepare(label: &str) -> String {
    format!(
        r#"(() => {{
  const query={};
  const controls=[...document.querySelectorAll(
    'button,a,[role="button"],[data-testid],[tabindex]:not([tabindex="-1"])'
  )];
  const root=controls.map(element=>{{
    const rect=element.getBoundingClientRect();
    const label=(element.getAttribute('aria-label')||element.innerText||
      element.textContent||'').replace(/\s+/g,' ').trim();
    return {{element,rect,label}};
  }}).filter(value=>value.rect.width>20&&value.rect.height>20&&
    value.label.toLowerCase().includes(query.toLowerCase()))
    .sort((left,right)=>Number(right.label===query)-Number(left.label===query)||
      right.rect.width*right.rect.height-left.rect.width*left.rect.height)[0]?.element;
  if(!root)return {{
    missing:true,
    labels:controls.map(element=>({{
      label:(element.getAttribute('aria-label')||element.innerText||
        element.textContent||'').replace(/\s+/g,' ').trim(),
      rect:Array.from(element.getClientRects()).length
    }})).filter(value=>value.label).slice(0,100)
  }};
  root.scrollIntoView({{block:'center',inline:'center',behavior:'instant'}});
  window.__recreateFidelityRoot=root;
  const rect=root.getBoundingClientRect();
  return {{x:rect.x+rect.width/2,y:rect.y+rect.height/2,label:
    (root.getAttribute('aria-label')||root.innerText||root.textContent||'').trim()}};
}})()"#,
        serde_json::to_string(label).expect("label should serialize")
    )
}

pub const SNAPSHOT: &str = r#"(() => {
  const root=window.__recreateFidelityRoot;
  if(!root)return null;
  const relative=element=>{
    if(element===root)return '.';
    const parts=[];
    for(let node=element;node&&node!==root;node=node.parentElement){
      const peers=[...node.parentElement.children].filter(value=>value.tagName===node.tagName);
      parts.unshift(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node)+1})`);
    }
    return parts.join('>');
  };
  const nodes=[root,...root.querySelectorAll('*')]
    .filter(element=>!element.closest('defs'))
    .map(element=>{
    const rect=element.getBoundingClientRect();
    const style=getComputedStyle(element);
    return {
      path:relative(element),
      tag:element.tagName.toLowerCase(),
      className:typeof element.className==='string'?element.className:
        element.getAttribute('class')||'',
      text:element.childElementCount===0?(element.textContent||''):'',
      rect:[rect.x,rect.y,rect.width,rect.height],
      style:{
        opacity:style.opacity,transform:style.transform,color:style.color,
        backgroundColor:style.backgroundColor,boxShadow:style.boxShadow,
        fill:style.fill,stroke:style.stroke,
        borderTopColor:style.borderTopColor,borderRightColor:style.borderRightColor,
        borderBottomColor:style.borderBottomColor,borderLeftColor:style.borderLeftColor
      }
    };
  });
  const animations=root.getAnimations({subtree:true}).map(animation=>{
    const timing=animation.effect?.getTiming?.()||{};
    const keyframes=animation.effect?.getKeyframes?.()||[];
    const properties=[...new Set(keyframes.flatMap(frame=>Object.keys(frame))
      .filter(key=>!['offset','easing','composite','computedOffset'].includes(key)))].sort();
    return {
      target:relative(animation.effect?.target),
      pseudo:animation.effect?.pseudoElement||null,
      currentTime:Number(animation.currentTime||0),
      duration:Number(timing.duration||0),
      delay:Number(timing.delay||0),
      easing:String(timing.easing||''),
      properties
    };
  });
  const rootRect=root.getBoundingClientRect();
  const hit=document.elementFromPoint(
    rootRect.x+rootRect.width/2,rootRect.y+rootRect.height/2
  );
  return {
    nodes,animations,
    document:[document.documentElement.scrollWidth,document.documentElement.scrollHeight],
    rootHovered:root.matches(':hover'),
    hitPath:hit&&root.contains(hit)?relative(hit):null,
    visibility:document.visibilityState
  };
})()"#;
