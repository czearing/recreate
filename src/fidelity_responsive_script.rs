pub fn snapshot(label: &str, initialize: bool, text_lock: &serde_json::Value) -> String {
    format!(
        r#"(() => {{
  const query={};
  const textLock={};
  for(const [path,values] of Object.entries(textLock)){{
    let element=null;
    try{{element=document.querySelector(path)}}catch{{}}
    if(!element)continue;
    const nodes=[...element.childNodes].filter(node=>node.nodeType===Node.TEXT_NODE);
    values.forEach((value,index)=>{{if(nodes[index])nodes[index].textContent=value}});
  }}
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
  if(!root)return null;
  const ancestors=[];
  for(let node=root;node&&node!==document.documentElement&&ancestors.length<10;
      node=node.parentElement)ancestors.push(node);
  if({}||!window.__recreateResponsiveRoot){{
    window.__recreateResponsiveRoot=root;
    window.__recreateResponsiveAncestors=ancestors;
  }}
  const styleNames=['display','flexDirection','flexWrap','flexGrow','flexShrink',
    'flexBasis','gap','rowGap','columnGap','width','minWidth','maxWidth',
    'height','minHeight','maxHeight','overflowX','overflowY','whiteSpace'];
  const nodeSnapshot=(element,depth)=>{{
    const rect=element.getBoundingClientRect(),style=getComputedStyle(element);
    return {{
      depth,tag:element.tagName.toLowerCase(),
      rect:[rect.x,rect.y,rect.width,rect.height],
      style:Object.fromEntries(styleNames.map(name=>[name,style[name]]))
    }};
  }};
  const relative=element=>{{
    if(element===root)return '.';
    const parts=[];
    for(let node=element;node&&node!==root;node=node.parentElement){{
      const peers=[...node.parentElement.children].filter(value=>value.tagName===node.tagName);
      parts.unshift(`${{node.tagName.toLowerCase()}}:nth-of-type(${{peers.indexOf(node)+1}})`);
    }}
    return parts.join('>');
  }};
  const text=[root,...root.querySelectorAll('*')].flatMap(element=>{{
    const direct=[...element.childNodes].filter(node=>node.nodeType===Node.TEXT_NODE&&
      node.textContent.trim());
    if(!direct.length)return [];
    const range=document.createRange();
    for(const node of direct)range.selectNodeContents(node);
    const rects=[...range.getClientRects()].filter(rect=>rect.width&&rect.height);
    return [{{path:relative(element),lines:rects.length,
      widths:rects.map(rect=>Math.round(rect.width*100)/100)}}];
  }});
  const flow=ancestors.flatMap((owner,ownerDepth)=>{{
    const values=[];
    for(let node=owner.previousElementSibling,offset=1;
        node&&offset<=6;node=node.previousElementSibling,offset++){{
      const descendants=[node,...node.querySelectorAll('*')].slice(0,80);
      for(const descendant of descendants){{
        const rect=descendant.getBoundingClientRect(),style=getComputedStyle(descendant);
        let path='.';
        if(descendant!==node){{
          const parts=[];
          for(let current=descendant;current&&current!==node;current=current.parentElement){{
            const peers=[...current.parentElement.children].filter(
              value=>value.tagName===current.tagName);
            parts.unshift(`${{current.tagName.toLowerCase()}}:nth-of-type(${{
              peers.indexOf(current)+1}})`);
          }}
          path=parts.join('>');
        }}
        values.push({{
          ownerDepth,offset,path,tag:descendant.tagName.toLowerCase(),
          className:typeof descendant.className==='string'?descendant.className:
            descendant.getAttribute('class')||'',
          text:(descendant.innerText||descendant.textContent||'').replace(/\s+/g,' ')
            .trim().slice(0,120),
          rect:[rect.x,rect.y,rect.width,rect.height],
          style:Object.fromEntries(['display','height','marginTop','marginBottom',
            'paddingTop','paddingBottom','position','fontFamily','fontSize','fontWeight',
            'lineHeight','letterSpacing','whiteSpace','wordBreak','overflowWrap']
            .map(name=>[name,style[name]]))
        }});
      }}
    }}
    return values;
  }});
  return {{
    identityStable:root===window.__recreateResponsiveRoot&&ancestors.every(
      (node,index)=>node===window.__recreateResponsiveAncestors[index]),
    ancestors:ancestors.map(nodeSnapshot),flow,text,
    document:[document.documentElement.scrollWidth,document.documentElement.scrollHeight]
  }};
}})()"#,
        serde_json::to_string(label).expect("label should serialize"),
        text_lock,
        initialize
    )
}

pub const TEXT_MAP: &str = r#"(() => {
  const pathOf=element=>{
    if(element===document.documentElement)return 'html';
    const parts=[];
    for(let node=element;node&&node.nodeType===1;node=node.parentElement){
      if(node===document.documentElement){parts.unshift('html');break}
      const peers=[...node.parentElement.children].filter(value=>value.tagName===node.tagName);
      parts.unshift(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node)+1})`);
    }
    return parts.join('>');
  };
  return Object.fromEntries([...document.querySelectorAll('*')].flatMap(element=>{
    const values=[...element.childNodes]
      .filter(node=>node.nodeType===Node.TEXT_NODE&&node.textContent.trim())
      .map(node=>node.textContent);
    return values.length?[[pathOf(element),values]]:[];
  }));
})()"#;
