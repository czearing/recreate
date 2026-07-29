use crate::{browser::Browser, probe_discovery};

const BEGIN: &str = r#"(() => {
  globalThis.__recreateOracleAction?.observer?.disconnect();
  const action={started:performance.now(),input:null,inputEvent:null,first:null,dirty:false,waited:false,
    observer:null,
    layoutObserver:null,scroll:null,focus:null,dirtyNodes:new Set(),dirtyTrees:new Set(),
    errors:0,unsafe:0,error:null,rejection:null,consoleError:console.error,
    animations:new Set(document.getAnimations()),
    timeouts:new Set(globalThis.__recreateOracle?.pending?.timeouts?.keys?.()||[])};
  action.mark=(node,tree=false)=>{
    if(node?.nodeType===Node.TEXT_NODE)node=node.parentElement;
    if(node?.nodeType===Node.ELEMENT_NODE)
      (tree?action.dirtyTrees:action.dirtyNodes).add(node);
  };
  action.changed=(node,tree=false)=>{
    action.dirty=true;action.first??=performance.now();action.mark(node,tree);
  };
  action.observer=new MutationObserver(records=>{
    action.dirty=true;
    action.first ??= performance.now();
    for(const record of records){
      action.mark(record.target);
      for(const node of record.addedNodes)action.mark(node,true);
    }
  });
  action.observer.observe(document,{attributes:true,childList:true,
    characterData:true,subtree:true});
  action.scroll=event=>action.changed(
    event.target===document?document.scrollingElement:event.target);
  action.focus=event=>action.changed(event.target);
  addEventListener('scroll',action.scroll,true);
  addEventListener('focusin',action.focus,true);
  action.inputEvent=()=>action.input??=performance.now();
  for(const type of ['mousemove','mousedown','keydown'])
    addEventListener(type,action.inputEvent,true);
  action.error=()=>action.errors++;
  action.rejection=()=>action.errors++;
  addEventListener('error',action.error);
  addEventListener('unhandledrejection',action.rejection);
  console.error=(...args)=>{action.errors++;action.consoleError.apply(console,args)};
  action.fetch=globalThis.fetch;
  globalThis.fetch=function(input,init){
    const method=String(init?.method||input?.method||'GET').toUpperCase();
    if(!['GET','HEAD','OPTIONS'].includes(method))action.unsafe++;
    return action.fetch.apply(this,arguments);
  };
  action.xhrOpen=XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open=function(method){
    if(!['GET','HEAD','OPTIONS'].includes(String(method).toUpperCase()))action.unsafe++;
    return action.xhrOpen.apply(this,arguments);
  };
  if(globalThis.PerformanceObserver){
    try{
      action.layoutObserver=new PerformanceObserver(list=>{
        for(const entry of list.getEntries())
          for(const source of entry.sources||[])action.changed(source.node);
      });
      action.layoutObserver.observe({type:'layout-shift',buffered:false});
    }catch{}
  }
  globalThis.__recreateOracleAction=action;
  globalThis.__recreateOracleActionResult=null;
})()"#;

pub(crate) async fn begin(browser: &mut Browser) -> anyhow::Result<()> {
    browser.cdp.evaluate(BEGIN).await?;
    Ok(())
}

pub(crate) async fn begin_at(browser: &mut Browser, anchor: &str) -> anyhow::Result<(f64, f64)> {
    let expression = format!(
        "(()=>{{const point=({})({});{};globalThis.__recreateOracleAction.mark(
          document.elementFromPoint(point.x,point.y),true);return point}})()",
        probe_discovery::FIND_ANCHOR,
        serde_json::to_string(anchor)?,
        BEGIN
    );
    let point = browser.cdp.evaluate(&expression).await?;
    let x = point["x"]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("source anchor is absent in candidate: {anchor}"))?;
    let y = point["y"]
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("source anchor has no hit point: {anchor}"))?;
    Ok((x, y))
}
