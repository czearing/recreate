use super::{
    attribute_sequences, interaction_scroll, interactions,
    jsx_attrs::{all_attributes, dynamic_attributes, jsx_tag, quoted, static_attributes, void_tag},
    jsx_variants, startup_overlays, structural_tree,
    tree::Components,
};
use crate::model::Specification;
use std::collections::BTreeMap;

pub fn app(
    specification: &Specification,
    components: &Components,
    class_maps: &[BTreeMap<String, String>],
    assets: &BTreeMap<String, String>,
) -> String {
    if specification.states.is_empty() {
        return "export default function App(){return null}\n".into();
    }
    let views = specification
        .states
        .iter()
        .zip(class_maps)
        .enumerate()
        .map(|(index, (state, classes))| {
            let mut handlers = interactions::base_handlers(specification, state);
            attribute_sequences::append_handlers(state, &mut handlers);
            let current = structural_tree::for_state(components, state, classes);
            let page = jsx_variants::page(state, &current, assets, &handlers);
            let startup = if state.startup_nodes.is_empty() {
                String::new()
            } else {
                let startup = structural_tree::fragment_nodes(&state.startup_nodes, classes);
                let fragment = jsx_variants::fragment(
                    &startup,
                    assets,
                    state.startup_delay_ms,
                    state.startup_duration_ms,
                );
                format!("{{showStartup?createPortal({fragment},document.body):null}}")
            };
            format!(
                "function Baseline{index}({{activate,showStartup,onStartupDone}}){{return <>{page}{startup}</>}}\n"
            )
        })
        .collect::<String>();
    let canonical = specification
        .states
        .iter()
        .enumerate()
        .max_by_key(|(_, state)| (state.nodes.len(), state.viewport.width))
        .map(|(index, _)| index)
        .unwrap_or_default();
    let view_names = (0..specification.states.len())
        .map(|index| format!("Baseline{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let widths = jsx_variants::widths(&specification.states);
    let state_imports = (1..=specification.interactions.len())
        .map(|index| format!("Interaction{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let state_overlay = (1..=specification.interactions.len())
        .map(|index| {
            format!("state==={index}?<Interaction{index} width={{width}} onReset={{reset}}/>:")
        })
        .collect::<String>()
        + "null";
    let closable = std::iter::once("false".to_string())
        .chain(specification.interactions.iter().map(|interaction| {
            interactions::closable(interaction, &specification.states).to_string()
        }))
        .collect::<Vec<_>>()
        .join(",");
    let scroll_targets = interaction_scroll::targets(specification);
    let attribute_sequences = attribute_sequences::javascript(specification);
    let output = format!(
        "import React,{{useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {} }} from './components/index.js';\nimport {{ {} }} from './states.jsx';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\nconst pathOf=element=>{{const parts=[];for(let node=element;node&&node!==document.documentElement;node=node.parentElement){{const peers=node.parentElement?[...node.parentElement.children].filter(child=>child.tagName===node.tagName):[node];parts.push(`${{node.tagName.toLowerCase()}}:nth-of-type(${{peers.indexOf(node)+1}})`)}}return `html>${{parts.reverse().join('>')}}`}};\nconst captureScroll=element=>{{const elements=[];for(let node=element?.parentElement;node&&node!==document.documentElement;node=node.parentElement){{if(node.scrollLeft||node.scrollTop)elements.push([pathOf(node),node.scrollLeft,node.scrollTop])}}return{{window:[scrollX,scrollY],elements}}}};\nconst restoreScroll=snapshot=>{{scrollTo(...snapshot.window);snapshot.elements.forEach(([path,left,top])=>{{const element=document.querySelector(path);if(element){{element.scrollLeft=left;element.scrollTop=top}}}})}};\n{}\nconst viewportWidths=[{widths}];\nconst closableStates=[{closable}];\nconst capturedScrolls={scroll_targets};\nconst attributeSequences={attribute_sequences};\nconst capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;\n        const subscribe=notify=>{{const media=viewportWidths.slice(1).map(width=>matchMedia(`(max-width:${{width}}px)`));media.forEach(query=>query.addEventListener('change',notify));addEventListener('resize',notify);return()=>{{media.forEach(query=>query.removeEventListener('change',notify));removeEventListener('resize',notify)}}}};\n{views}const baselineViews=[{view_names}];\n        export default function App(){{const[state,setState]=useState(0);const[scrollRevision,setScrollRevision]=useState(0);const lastTrigger=useRef('');const scroll=useRef(null);const width=useSyncExternalStore(subscribe,()=>document.documentElement.clientWidth,()=>0);const viewport=selectViewport(width,viewportWidths);const View=baselineViews[{canonical}];const reset=()=>{{const selector='[data-recreate-trigger=\"'+lastTrigger.current+'\"]';scroll.current=captureScroll(document.querySelector(selector));setState(0);requestAnimationFrame(()=>document.querySelector(selector)?.focus({{preventScroll:true}}))}};const activate=(event,next)=>{{lastTrigger.current=event.currentTarget.dataset.recreateTrigger;const captured=capturedScroll(next,viewport);if(!closableStates[next]){{scroll.current=captured;if(captured)setScrollRevision(value=>value+1);return}}if(state===next){{reset();return}}scroll.current=event.currentTarget.dataset.recreatePreserveScroll==='false'?(captured??{{window:[0,0],elements:[]}}):captureScroll(event.currentTarget);setState(next)}};useLayoutEffect(()=>{{document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>element.setAttribute('aria-expanded',String(Number(element.dataset.recreateTrigger)===state)));if(!scroll.current)return;restoreScroll(scroll.current);requestAnimationFrame(()=>restoreScroll(scroll.current))}},[state,scrollRevision]);useEffect(()=>{{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{{const element=document.querySelector(`[data-recreate-sequence=\"${{index}}\"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])}},sequence.interval_ms)}});return()=>timers.forEach(timer=>timer&&clearInterval(timer))}},[viewport]);useEffect(()=>{{if(!state||!closableStates[state])return;const key=event=>{{if(event.key==='Escape')reset()}};const pointer=event=>{{if(!event.target.closest('[data-recreate-surface],[data-recreate-control]'))reset()}};addEventListener('keydown',key);addEventListener('pointerdown',pointer);return()=>{{removeEventListener('keydown',key);removeEventListener('pointerdown',pointer)}}}},[state]);const overlay={state_overlay};return <><View activate={{activate}}/>{{overlay}}</>}}\n",
        components
            .items
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        state_imports,
        jsx_variants::selector(),
    );
    let output = output.replace(
        "const capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;",
        "const mergeHorizontalScroll=(current,captured)=>{const live=new Map(current.elements.map(value=>[value[0],value]));const paths=new Set(captured.elements.map(value=>value[0]));return{window:current.window,elements:[...captured.elements.map(([path,left,top])=>[path,left,live.get(path)?.[2]??top]),...current.elements.filter(([path])=>!paths.has(path))]}};\nconst capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;",
    );
    let output = output.replace(
        "scroll.current=captured;if(captured)setScrollRevision(value=>value+1)",
        "scroll.current=captured?mergeHorizontalScroll(captureScroll(event.currentTarget),captured):null;if(scroll.current)setScrollRevision(value=>value+1)",
    );
    let output = output.replace(
        "if(!scroll.current)return;restoreScroll(scroll.current);",
        "if(state&&closableStates[state])requestAnimationFrame(()=>{const surface=document.querySelector('[data-recreate-surface]');const target=document.querySelector('[data-recreate-surface]:is(input,button,[tabindex]),[data-recreate-surface] input,[data-recreate-surface] button,[data-recreate-surface] [tabindex]')||surface;if(target){if(target===surface&&!target.matches('input,button,[tabindex]'))target.tabIndex=-1;target.focus({preventScroll:true})}});if(!scroll.current)return;restoreScroll(scroll.current);",
    );
    startup_overlays::runtime(attribute_sequences::runtime(output), &specification.states)
}

pub fn component(
    component: &super::tree::Component,
    components: &Components,
    assets: &BTreeMap<String, String>,
) -> String {
    let Some(root) = component.roots.first() else {
        return String::new();
    };
    let Some(node) = components.nodes.get(root) else {
        return String::new();
    };
    let class = components.classes.get(root).cloned().unwrap_or_default();
    let attributes = static_attributes(node, assets);
    format!(
        "import React from 'react';\nexport default function {}({{children,...props}}){{return <{} className={}{} {{...props}}>{{children}}</{}>}}\n",
        component.name,
        jsx_tag(&node.tag),
        quoted(&class),
        attributes,
        jsx_tag(&node.tag)
    )
}

pub(super) fn render(
    path: &str,
    components: &Components,
    assets: &BTreeMap<String, String>,
    depth: usize,
    allow_component: bool,
    handlers: &BTreeMap<String, String>,
) -> String {
    let Some(node) = components.nodes.get(path) else {
        return String::new();
    };
    let indent = "  ".repeat(depth);
    if node.tag == "#text" {
        return format!(
            "{indent}{{{}}}\n",
            serde_json::to_string(&node.text).unwrap()
        );
    }
    let children = components
        .children
        .get(path)
        .into_iter()
        .flatten()
        .map(|child| render(child, components, assets, depth + 1, true, handlers))
        .collect::<String>();
    if allow_component && let Some(index) = components.by_root.get(path) {
        let name = &components.items[*index].name;
        let attributes = dynamic_attributes(node, assets);
        return format!(
            "{indent}<{name}{attributes}{}>\n{}{indent}</{name}>\n",
            event(path, handlers),
            children
        );
    }
    let class = components.classes.get(path).cloned().unwrap_or_default();
    let attributes = all_attributes(node, assets);
    if void_tag(&node.tag) {
        return format!(
            "{indent}<{} className={}{}{} />\n",
            jsx_tag(&node.tag),
            quoted(&class),
            attributes,
            event(path, handlers)
        );
    }
    format!(
        "{indent}<{} className={}{}{}>\n{}{indent}</{}>\n",
        jsx_tag(&node.tag),
        quoted(&class),
        attributes,
        event(path, handlers),
        children,
        jsx_tag(&node.tag)
    )
}

fn event(path: &str, handlers: &BTreeMap<String, String>) -> String {
    handlers
        .get(path)
        .map(|value| format!(" {value}"))
        .unwrap_or_default()
}
