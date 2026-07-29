pub const CAPTURE: &str = "globalThis.__recreateOracleCapture()";
pub const FUNCTION: &str = r#"() => {
  const round=value=>Math.round(Number(value)*100)/100;
  const styleNames=['display','visibility','opacity','color','background-color',
    'background-image','border-top-color','border-right-color','border-bottom-color',
    'border-left-color','border-top-width','border-right-width','border-bottom-width',
    'border-left-width','border-radius','box-shadow','outline-color','outline-style',
    'outline-width','outline-offset','transform','translate','scale','rotate','position',
    'overflow','overflow-x','overflow-y','font-family','font-size','font-weight',
    'line-height','text-decoration','cursor','pointer-events'];
  const actionSelector='a[href],button,input:not([type="hidden"]),select,textarea,summary,'+
    '[role="button"],[role="tab"],[role="menuitem"],[role="option"],[role="checkbox"],'+
    '[role="radio"],[role="switch"],[role="slider"],[contenteditable="true"],'+
    '[tabindex]:not([tabindex="-1"])';
  const visible=element=>{
    const style=getComputedStyle(element),rect=element.getBoundingClientRect();
    return style.display!=='none'&&style.visibility!=='hidden'&&
      Number(style.opacity||1)>=0.01&&rect.width>0&&rect.height>0;
  };
  const semantic=element=>{
    const style=getComputedStyle(element);
    return /^(a|button|input|select|textarea|summary|img|svg|canvas|video|audio)$/
      .test(element.localName)||element.hasAttribute('role')||
      element.hasAttribute('aria-label')||element.hasAttribute('alt')||
      (!element.children.length&&(element.textContent||'').trim())||
      style.backgroundColor!=='rgba(0, 0, 0, 0)'||style.boxShadow!=='none'||
      ['borderTopWidth','borderRightWidth','borderBottomWidth','borderLeftWidth']
        .some(name=>parseFloat(style[name])>0)||element.scrollWidth>element.clientWidth||
      element.scrollHeight>element.clientHeight;
  };
  const key=element=>[
    element.getAttribute('role')||'',element.getAttribute('aria-label')||
      element.getAttribute('alt')||'',
    '',
    element.localName
  ].join('|');
  const occurrences=new Map();
  const nodes=[...document.querySelectorAll('*')].filter(element=>
    visible(element)&&semantic(element)).map(element=>{
    const anchorKey=key(element),occurrence=occurrences.get(anchorKey)||0;
    occurrences.set(anchorKey,occurrence+1);
    const rect=element.getBoundingClientRect(),style=getComputedStyle(element);
    const source=element.getAttribute('src')||element.getAttribute('href')||'';
    const asset=source.startsWith('blob:')?'blob:':source;
    return {
      anchor:`${anchorKey}@${occurrence}`,tag:element.localName,
      actionable:element.matches(actionSelector),
      role:element.getAttribute('role')||'',name:element.getAttribute('aria-label')||
        element.getAttribute('alt')||'',
      text:element.children.length?'':(element.textContent||'').replace(/\s+/g,' ').trim(),
      state:Object.fromEntries(['aria-expanded','aria-selected','aria-pressed',
        'aria-checked','aria-disabled','disabled','hidden','value','checked']
        .map(name=>[name,element.getAttribute(name)??element[name]??null])),
      rect:[round(rect.x),round(rect.y),round(rect.width),round(rect.height)],
      scroll:[round(element.scrollLeft),round(element.scrollTop),
        round(element.scrollWidth),round(element.scrollHeight)],
      style:Object.fromEntries(styleNames.map(name=>[name,style.getPropertyValue(name)])),
      asset
    };
  });
  const active=document.activeElement;
  const focus=active&&active!==document.body?(()=>{
    const wanted=key(active),matches=nodes.filter(node=>node.anchor.startsWith(`${wanted}@`));
    const index=[...document.querySelectorAll('*')].filter(element=>
      visible(element)&&semantic(element)&&key(element)===wanted).indexOf(active);
    return matches[index]?.anchor||'';
  })():'';
  const animations=document.getAnimations({subtree:true}).map(animation=>({
    playState:animation.playState,playbackRate:animation.playbackRate,
    timing:animation.effect?.getComputedTiming(),
    keyframes:animation.effect?.getKeyframes()
  }));
  return {nodes,focus,document:[document.documentElement.scrollWidth,
    document.documentElement.scrollHeight,scrollX,scrollY],animations,
    action:globalThis.__recreateOracleActionResult||null};
}"#;
