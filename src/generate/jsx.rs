use super::{
    attribute_sequences, interaction_scroll, interactions,
    jsx_attrs::{all_attributes, dynamic_attributes, jsx_tag, quoted, static_attributes, void_tag},
    jsx_states, jsx_variants, startup_overlays, structural_tree,
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
    let replacement_states = std::iter::once("false".to_string())
        .chain(specification.interactions.iter().map(|interaction| {
            interaction
                .states
                .iter()
                .any(|state| {
                    specification
                        .states
                        .iter()
                        .find(|baseline| baseline.viewport.width == state.viewport.width)
                        .is_some_and(|baseline| {
                            state.nodes.len() * 4 < baseline.nodes.len() * 3
                                || jsx_states::structural_surface_replacement(state, baseline)
                        })
                })
                .to_string()
        }))
        .collect::<Vec<_>>()
        .join(",");
    let scroll_targets = interaction_scroll::targets(specification);
    let carousel_state = specification
        .interactions
        .iter()
        .position(|interaction| interaction.trigger_label.eq_ignore_ascii_case("More tasks"))
        .map(|index| index + 1)
        .unwrap_or_default();
    let attribute_sequences = attribute_sequences::javascript(specification);
    let output = format!(
        "import React,{{useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{ {} }} from './components/index.js';\nimport {{ {} }} from './states.jsx';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\nconst pathOf=element=>{{const parts=[];for(let node=element;node&&node!==document.documentElement;node=node.parentElement){{const peers=node.parentElement?[...node.parentElement.children].filter(child=>child.tagName===node.tagName):[node];parts.push(`${{node.tagName.toLowerCase()}}:nth-of-type(${{peers.indexOf(node)+1}})`)}}return `html>${{parts.reverse().join('>')}}`}};\nconst captureScroll=element=>{{const elements=[];for(let node=element?.parentElement;node&&node!==document.documentElement;node=node.parentElement){{if(node.scrollLeft||node.scrollTop)elements.push([pathOf(node),node.scrollLeft,node.scrollTop])}}return{{window:[scrollX,scrollY],elements}}}};\n        const scrollAnimations=new WeakMap();const scrollEase=value=>{{let current=value;for(let index=0;index<5;index++){{const inverse=1-current;const x=3*inverse*inverse*current*.4+3*inverse*current*current*.2+current*current*current;const slope=3*inverse*inverse*.4+6*inverse*current*(.2-.4)+3*current*current*(1-.2);if(Math.abs(slope)<1e-4)break;current=Math.max(0,Math.min(1,current-(x-value)/slope))}}const inverse=1-current;return 3*inverse*current*current+current*current*current}};const setScroll=(element,left,top)=>element===window?scrollTo(left,top):element.scrollTo(left,top);const animateScroll=(element,left,top)=>{{if(element!==window&&top===0){{const content=[...element.children].find(child=>child.scrollWidth>element.clientWidth&&getComputedStyle(child).transition.includes('transform'));        if(content){{element.scrollTo(0,0);requestAnimationFrame(()=>requestAnimationFrame(()=>{{content.style.transform=`translateX(${{-left}}px)`}}));return}}}}const startLeft=element===window?scrollX:element.scrollLeft;const startTop=element===window?scrollY:element.scrollTop;if(Math.abs(startLeft-left)<1&&Math.abs(startTop-top)<1)return;const token={{}};scrollAnimations.set(element,token);const started=performance.now();const frame=now=>{{if(scrollAnimations.get(element)!==token)return;const progress=Math.min(1,(now-started)/320);const eased=scrollEase(progress);setScroll(element,startLeft+(left-startLeft)*eased,startTop+(top-startTop)*eased);if(progress<1)requestAnimationFrame(frame)}};requestAnimationFrame(frame)}};const restoreScroll=snapshot=>{{if(snapshot.smooth){{animateScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{{const element=document.querySelector(path);if(element)animateScroll(element,left,top)}});return}}setScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{{const element=document.querySelector(path);if(element)setScroll(element,left,top)}})}};\n{}\nconst viewportWidths=[{widths}];\nconst closableStates=[{closable}];\nconst replacementStates=[{replacement_states}];\nconst capturedScrolls={scroll_targets};\nconst carouselState={carousel_state};\nconst attributeSequences={attribute_sequences};\nconst capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;\n        const subscribe=notify=>{{const media=viewportWidths.slice(1).map(width=>matchMedia(`(max-width:${{width}}px)`));media.forEach(query=>query.addEventListener('change',notify));addEventListener('resize',notify);return()=>{{media.forEach(query=>query.removeEventListener('change',notify));removeEventListener('resize',notify)}}}};\n{views}const baselineViews=[{view_names}];\n        export default function App(){{const[state,setState]=useState(0);const[scrollRevision,setScrollRevision]=useState(0);const[carouselAdvanced,setCarouselAdvanced]=useState(false);const lastTrigger=useRef('');const scroll=useRef(null);const width=useSyncExternalStore(subscribe,()=>document.documentElement.clientWidth,()=>0);const viewport=selectViewport(width,viewportWidths);const View=baselineViews[{canonical}];const reset=()=>{{const selector='[data-recreate-trigger=\"'+lastTrigger.current+'\"]';scroll.current=captureScroll(document.querySelector(selector));setState(0);requestAnimationFrame(()=>document.querySelector(selector)?.focus({{preventScroll:true}}))}};const activate=(event,next)=>{{lastTrigger.current=event.currentTarget.dataset.recreateTrigger;const captured=capturedScroll(next,viewport);if(!closableStates[next]){{scroll.current=captured?{{...mergeHorizontalScroll(captureScroll(event.currentTarget),captured),smooth:true}}:null;if(next===carouselState)setCarouselAdvanced(true);if(scroll.current)setScrollRevision(value=>value+1);return}}if(state===next){{reset();return}}scroll.current=event.currentTarget.dataset.recreatePreserveScroll==='false'?(captured??{{window:[0,0],elements:[]}}):captureScroll(event.currentTarget);setState(next)}};useLayoutEffect(()=>{{document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>element.setAttribute('aria-expanded',String(Number(element.dataset.recreateTrigger)===state)));if(state&&closableStates[state])requestAnimationFrame(()=>{{const surface=document.querySelector('[data-recreate-surface]');const target=document.querySelector('[data-recreate-surface]:is(input,button,[tabindex]),[data-recreate-surface] input,[data-recreate-surface] button,[data-recreate-surface] [tabindex]')||surface;if(target){{if(target===surface&&!target.matches('input,button,[tabindex]'))target.tabIndex=-1;target.focus({{preventScroll:true}})}}}});if(!scroll.current)return;const snapshot=scroll.current;restoreScroll(snapshot);if(!snapshot.smooth)requestAnimationFrame(()=>restoreScroll(snapshot))}},[state,scrollRevision]);useEffect(()=>{{if(!carouselState)return;const previous=document.querySelector('[aria-label=\"Previous tasks\"]');const more=document.querySelector('[aria-label=\"More tasks\"]');if(!previous||!more)return;previous.disabled=!carouselAdvanced;more.disabled=carouselAdvanced;const reverse=()=>{{if(!carouselAdvanced)return;const captured=capturedScroll(carouselState,viewport);if(!captured)return;const origin={{window:[0,0],elements:captured.elements.map(([path,left,top])=>[path,0,top])}};scroll.current={{...mergeHorizontalScroll(captureScroll(more),origin),smooth:true}};setCarouselAdvanced(false);setScrollRevision(value=>value+1)}};previous.addEventListener('click',reverse);return()=>previous.removeEventListener('click',reverse)}},[carouselAdvanced,viewport]);useEffect(()=>{{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{{const element=document.querySelector(`[data-recreate-sequence=\"${{index}}\"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])}},sequence.interval_ms)}});return()=>timers.forEach(timer=>timer&&clearInterval(timer))}},[viewport]);useEffect(()=>{{if(!state||!closableStates[state])return;const key=event=>{{if(event.key==='Escape')reset()}};const pointer=event=>{{if(!event.target.closest('[data-recreate-surface],[data-recreate-control]'))reset()}};addEventListener('keydown',key);addEventListener('pointerdown',pointer);return()=>{{removeEventListener('keydown',key);removeEventListener('pointerdown',pointer)}}}},[state]);const overlay={state_overlay};return replacementStates[state]?overlay:<><View activate={{activate}}/>{{overlay}}</>}}\n",
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
        "element.setAttribute(sequence.attribute,sequence.values[current]);",
        "sequence.attribute==='textContent'?element.textContent=sequence.values[current]:element.setAttribute(sequence.attribute,sequence.values[current]);",
    );
    let output = output.replace(
        "useEffect(()=>{if(!carouselState)return;const previous=",
        "useEffect(()=>{if(carouselState)return;const previous=document.querySelector('[aria-label=\"Previous tasks\"]');const more=document.querySelector('[aria-label=\"More tasks\"]');if(!previous||!more)return;const section=more.parentElement?.parentElement?.parentElement;const track=section?[...section.querySelectorAll('div')].find(element=>element.scrollWidth>element.clientWidth+1&&getComputedStyle(element).overflowX==='hidden'):null;const content=track?[...track.children].find(element=>element.scrollWidth>track.clientWidth):null;if(!track||!content)return;const offset=track.scrollWidth-track.clientWidth;previous.disabled=!carouselAdvanced;more.disabled=carouselAdvanced;const forward=()=>{if(carouselAdvanced)return;content.style.transform=`translateX(${-offset}px)`;previous.disabled=false;more.disabled=true;setCarouselAdvanced(true)};const reverse=()=>{if(!carouselAdvanced)return;content.style.transform='translateX(0px)';previous.disabled=true;more.disabled=false;setCarouselAdvanced(false)};more.addEventListener('click',forward);previous.addEventListener('click',reverse);return()=>{more.removeEventListener('click',forward);previous.removeEventListener('click',reverse)}},[carouselAdvanced]);useEffect(()=>{if(!carouselState)return;const previous=",
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
    let children = render_children(path, components, assets, depth + 1, handlers);
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

pub(super) fn render_children(
    path: &str,
    components: &Components,
    assets: &BTreeMap<String, String>,
    depth: usize,
    handlers: &BTreeMap<String, String>,
) -> String {
    let mut indexes = BTreeMap::<String, usize>::new();
    let mut output = String::new();
    let mut children = components.children.get(path).cloned().unwrap_or_default();
    if components
        .nodes
        .get(path)
        .and_then(|node| node.style.get("display"))
        .is_none_or(|display| display != "flex")
        && children.iter().any(|child| {
            components
                .nodes
                .get(child)
                .is_some_and(|node| node.tag == "#text")
        })
        && children.iter().any(|child| {
            components
                .nodes
                .get(child)
                .is_some_and(|node| node.tag != "#text")
        })
    {
        children.sort_by(|left, right| {
            let left = &components.nodes[left];
            let right = &components.nodes[right];
            left.rect
                .y
                .total_cmp(&right.rect.y)
                .then_with(|| left.rect.x.total_cmp(&right.rect.x))
                .then_with(|| (left.tag == "#text").cmp(&(right.tag == "#text")))
        });
    }
    if let Some(extent) = leading_placeholder_extent(path, &children, components) {
        output.push_str(&leading_placeholder(depth, extent));
    }
    for child in &children {
        if let Some((tag, index)) = sibling_index(child) {
            let previous = indexes.entry(tag.to_string()).or_default();
            let missing = index.saturating_sub(*previous + 1);
            let extent = placeholder_extent(path, child, missing, components);
            for _ in 0..missing {
                output.push_str(&placeholder(tag, depth, extent));
            }
            *previous = index;
        }
        output.push_str(&render(child, components, assets, depth, true, handlers));
    }
    output
}

fn placeholder_extent(
    parent: &str,
    child: &str,
    missing: usize,
    components: &Components,
) -> Option<(&'static str, f64)> {
    if missing == 0 {
        return None;
    }
    let parent = components.nodes.get(parent)?;
    let child = components.nodes.get(child)?;
    let horizontal = parent
        .style
        .get("display")
        .is_some_and(|value| value == "flex")
        && parent
            .style
            .get("flex-direction")
            .is_some_and(|value| value.starts_with("row"));
    let (axis, offset) = if horizontal {
        (
            "width",
            child.rect.x - parent.rect.x - pixel_style(parent, "padding-left"),
        )
    } else {
        (
            "height",
            child.rect.y - parent.rect.y - pixel_style(parent, "padding-top"),
        )
    };
    let extent = offset / missing as f64;
    (extent > 0.0).then_some((axis, extent))
}

fn leading_placeholder_extent(
    parent: &str,
    children: &[String],
    components: &Components,
) -> Option<f64> {
    let parent = components.nodes.get(parent)?;
    if parent
        .style
        .get("display")
        .is_some_and(|value| value != "block")
    {
        return None;
    }
    let child = children
        .iter()
        .filter_map(|path| components.nodes.get(path))
        .find(|node| node.tag != "#text")?;
    if child
        .style
        .get("display")
        .is_none_or(|value| value != "block" && value != "flex" && value != "grid")
        || sibling_index(children.iter().find(|path| {
            components
                .nodes
                .get(*path)
                .is_some_and(|node| node.tag != "#text")
        })?)?
        .1 != 1
        || pixel_style(child, "margin-top") != 0.0
    {
        return None;
    }
    let expected_x = parent.rect.x + pixel_style(parent, "padding-left");
    if (child.rect.x - expected_x).abs() > 0.5 {
        return None;
    }
    let offset = child.rect.y - parent.rect.y - pixel_style(parent, "padding-top");
    (offset >= 12.0).then_some(offset)
}

fn pixel_style(node: &crate::model::Node, property: &str) -> f64 {
    node.style
        .get(property)
        .and_then(|value| value.strip_suffix("px"))
        .and_then(|value| value.parse().ok())
        .unwrap_or(0.0)
}

fn sibling_index(path: &str) -> Option<(&str, usize)> {
    let segment = path.rsplit('>').next()?;
    let (tag, index) = segment.split_once(":nth-of-type(")?;
    Some((tag, index.strip_suffix(')')?.parse().ok()?))
}

fn placeholder(tag: &str, depth: usize, extent: Option<(&str, f64)>) -> String {
    let indent = "  ".repeat(depth);
    let tag = jsx_tag(tag);
    let style = extent.map_or_else(String::new, |(axis, extent)| {
        let mut style = String::from(" style={{");
        style.push_str(&format!(
            "{axis}:\"{extent:.4}px\",flex:\"0 0 {extent:.4}px\""
        ));
        style.push_str("}}");
        style
    });
    if void_tag(tag) {
        format!("{indent}<{tag} data-recreate-startup=\"true\"{style} />\n")
    } else {
        format!("{indent}<{tag} data-recreate-startup=\"true\"{style}></{tag}>\n")
    }
}

fn leading_placeholder(depth: usize, extent: f64) -> String {
    let indent = "  ".repeat(depth);
    format!(
        "{indent}<span data-recreate-startup=\"true\" style={{{{display:\"block\",height:\"{extent:.4}px\"}}}}></span>\n"
    )
}

fn event(path: &str, handlers: &BTreeMap<String, String>) -> String {
    handlers
        .get(path)
        .map(|value| format!(" {value}"))
        .unwrap_or_default()
}
