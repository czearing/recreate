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
    let replacement_states = std::iter::once("false".to_string())
        .chain(
            specification
                .interactions
                .iter()
                .map(|_| "false".to_string()),
        )
        .collect::<Vec<_>>()
        .join(",");
    let stateful = std::iter::once("false".to_string())
        .chain(specification.interactions.iter().map(|interaction| {
            interactions::rendered(interaction, &specification.states).to_string()
        }))
        .collect::<Vec<_>>()
        .join(",");
    let focused_targets = std::iter::once("null".to_string())
        .chain(specification.interactions.iter().map(|interaction| {
            interaction
                .focused_path
                .as_ref()
                .map(|path| serde_json::to_string(path).unwrap())
                .unwrap_or_else(|| "null".into())
        }))
        .collect::<Vec<_>>()
        .join(",");
    let scroll_targets = interaction_scroll::targets(specification);
    let carousel = specification
        .interactions
        .iter()
        .enumerate()
        .find(|(_, interaction)| {
            interaction_scroll::moves_horizontally(interaction, &specification.states)
        });
    let carousel_state = carousel.map_or(0, |(index, _)| index + 1);
    let (carousel_previous, carousel_next) = carousel
        .and_then(|(_, interaction)| {
            let baseline = specification.states.first()?;
            let trigger = baseline
                .nodes
                .iter()
                .find(|node| node.path == interaction.trigger_path)?;
            let previous = baseline
                .nodes
                .iter()
                .filter(|node| {
                    node.parent == trigger.parent
                        && node.path != trigger.path
                        && matches!(node.tag.as_str(), "button" | "input")
                })
                .find(|node| {
                    node.attributes.contains_key("disabled")
                        || node
                            .attributes
                            .get("aria-disabled")
                            .is_some_and(|value| value == "true")
                })
                .or_else(|| {
                    baseline.nodes.iter().find(|node| {
                        node.parent == trigger.parent
                            && node.path != trigger.path
                            && matches!(node.tag.as_str(), "button" | "input")
                    })
                })?;
            Some((previous.path.clone(), trigger.path.clone()))
        })
        .unwrap_or_default();
    let attribute_sequences = attribute_sequences::javascript(specification);
    let responsive_attributes =
        super::responsive_attributes::javascript(&specification.states, canonical);
    let initial_scrolls = super::initial_scroll::targets(specification);
    let output = format!(
        "import React,{{useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{moveCarousel}} from './runtime/carousel.mjs';\nimport {{reduceInteraction}} from './runtime/interaction.mjs';\nimport {{startSequences}} from './runtime/sequence.mjs';\nimport {{ {} }} from './components/index.js';\nimport {{ {} }} from './states.jsx';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\nconst pathOf=element=>{{const parts=[];for(let node=element;node&&node!==document.documentElement;node=node.parentElement){{const peers=node.parentElement?[...node.parentElement.children].filter(child=>child.tagName===node.tagName):[node];parts.push(`${{node.tagName.toLowerCase()}}:nth-of-type(${{peers.indexOf(node)+1}})`)}}return `html>${{parts.reverse().join('>')}}`}};\nconst captureScroll=element=>{{const elements=[];for(let node=element?.parentElement;node&&node!==document.documentElement;node=node.parentElement){{if(node.scrollLeft||node.scrollTop)elements.push([pathOf(node),node.scrollLeft,node.scrollTop])}}return{{window:[scrollX,scrollY],elements}}}};\n        const scrollAnimations=new WeakMap();const scrollEase=value=>{{let current=value;for(let index=0;index<5;index++){{const inverse=1-current;const x=3*inverse*inverse*current*.4+3*inverse*current*current*.2+current*current*current;const slope=3*inverse*inverse*.4+6*inverse*current*(.2-.4)+3*current*current*(1-.2);if(Math.abs(slope)<1e-4)break;current=Math.max(0,Math.min(1,current-(x-value)/slope))}}const inverse=1-current;return 3*inverse*current*current+current*current*current}};const setScroll=(element,left,top)=>element===window?scrollTo(left,top):element.scrollTo(left,top);const animateScroll=(element,left,top)=>{{if(element!==window&&top===0){{const content=[...element.children].find(child=>child.scrollWidth>element.clientWidth&&getComputedStyle(child).transition.includes('transform'));        if(content){{element.scrollTo(0,0);requestAnimationFrame(()=>requestAnimationFrame(()=>{{content.style.transform=`translateX(${{-left}}px)`}}));return}}}}const startLeft=element===window?scrollX:element.scrollLeft;const startTop=element===window?scrollY:element.scrollTop;if(Math.abs(startLeft-left)<1&&Math.abs(startTop-top)<1)return;const token={{}};scrollAnimations.set(element,token);const started=performance.now();const frame=now=>{{if(scrollAnimations.get(element)!==token)return;const progress=Math.min(1,(now-started)/320);const eased=scrollEase(progress);setScroll(element,startLeft+(left-startLeft)*eased,startTop+(top-startTop)*eased);if(progress<1)requestAnimationFrame(frame)}};requestAnimationFrame(frame)}};const restoreScroll=snapshot=>{{if(snapshot.smooth){{animateScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{{const element=document.querySelector(path);if(element)animateScroll(element,left,top)}});return}}setScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{{const element=document.querySelector(path);if(element)setScroll(element,left,top)}})}};\n{}\nconst viewportWidths=[{widths}];\nconst closableStates=[{closable}];\nconst statefulStates=[{stateful}];\nconst replacementStates=[{replacement_states}];\nconst capturedScrolls={scroll_targets};\nconst carouselState={carousel_state};\nconst attributeSequences={attribute_sequences};\nconst responsiveAttributes={responsive_attributes};\nconst capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;\n        const subscribe=notify=>{{const media=viewportWidths.slice(1).map(width=>matchMedia(`(max-width:${{width}}px)`));media.forEach(query=>query.addEventListener('change',notify));addEventListener('resize',notify);return()=>{{media.forEach(query=>query.removeEventListener('change',notify));removeEventListener('resize',notify)}}}};\n{views}const baselineViews=[{view_names}];\n                        export default function App(){{const[state,setState]=useState(0);const[scrollRevision,setScrollRevision]=useState(0);const[carouselAdvanced,setCarouselAdvanced]=useState(false);const lastTrigger=useRef('');const scroll=useRef(null);const width=useSyncExternalStore(subscribe,()=>document.documentElement.clientWidth,()=>0);const viewport=selectViewport(width,viewportWidths);const reset=()=>{{const selector='[data-recreate-trigger=\"'+lastTrigger.current+'\"]';scroll.current=captureScroll(document.querySelector(selector));setState(0);requestAnimationFrame(()=>document.querySelector(selector)?.focus({{preventScroll:true}}))}};const activate=(event,next,inputActive)=>{{if(inputActive===false){{if(state===next)reset();return}}if(inputActive===true&&state===next)return;lastTrigger.current=event.currentTarget.dataset.recreateTrigger;const captured=capturedScroll(next,viewport);if(!statefulStates[next]){{scroll.current=captured?{{...mergeHorizontalScroll(captureScroll(event.currentTarget),captured),smooth:true}}:null;if(next===carouselState)setCarouselAdvanced(true);if(scroll.current)setScrollRevision(value=>value+1);return}}if(state===next){{reset();return}}scroll.current=event.currentTarget.dataset.recreatePreserveScroll==='false'?(captured??{{window:[0,0],elements:[]}}):captureScroll(event.currentTarget);setState(next)}};useLayoutEffect(()=>{{if(state!==0)return;for(const[path,attributes]of responsiveAttributes[viewport]||[]){{const element=document.querySelector(path);if(!element)continue;for(const[name,value]of attributes)value===null?element.removeAttribute(name):element.setAttribute(name,value)}}}},[viewport,state]);useLayoutEffect(()=>{{document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>element.setAttribute('aria-expanded',String(Number(element.dataset.recreateTrigger)===state)));if(state&&closableStates[state])requestAnimationFrame(()=>{{const surface=document.querySelector('[data-recreate-surface]');const target=document.querySelector('[data-recreate-surface]:is(input,button,[tabindex]),[data-recreate-surface] input,[data-recreate-surface] button,[data-recreate-surface] [tabindex]')||surface;if(target){{if(target===surface&&!target.matches('input,button,[tabindex]'))target.tabIndex=-1;target.focus({{preventScroll:true}})}}}});if(!scroll.current)return;const snapshot=scroll.current;restoreScroll(snapshot);if(!snapshot.smooth)requestAnimationFrame(()=>restoreScroll(snapshot))}},[state,scrollRevision]);useEffect(()=>{{if(!carouselState)return;const previous=document.querySelector(carouselPrevious);const more=document.querySelector(carouselNext);if(!previous||!more)return;previous.disabled=!carouselAdvanced;more.disabled=carouselAdvanced;const reverse=()=>{{if(!carouselAdvanced)return;const captured=capturedScroll(carouselState,viewport);if(!captured)return;const origin={{window:[0,0],elements:captured.elements.map(([path,left,top])=>[path,0,top])}};scroll.current={{...mergeHorizontalScroll(captureScroll(more),origin),smooth:true}};setCarouselAdvanced(false);setScrollRevision(value=>value+1)}};previous.addEventListener('click',reverse);return()=>previous.removeEventListener('click',reverse)}},[carouselAdvanced,viewport]);useEffect(()=>{{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{{const element=document.querySelector(`[data-recreate-sequence=\"${{index}}\"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])}},sequence.interval_ms)}});return()=>timers.forEach(timer=>timer&&clearInterval(timer))}},[viewport]);useEffect(()=>{{if(!state||!closableStates[state])return;const key=event=>{{if(event.key==='Escape')reset()}};const pointer=event=>{{if(!event.target.closest('[data-recreate-surface],[data-recreate-control]'))reset()}}        ;addEventListener('keydown',key);addEventListener('pointerdown',pointer);return()=>{{removeEventListener('keydown',key);removeEventListener('pointerdown',pointer)}}}},[state]);const overlay={state_overlay};const baseline=baselineViews[viewport]({{activate,showStartup:!startupDone,onStartupDone:()=>setStartupDone(true)}});return replacementStates[state]?overlay:<>{{baseline}}{{overlay}}</>}}\n",
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
        "const mergeHorizontalScroll=(current,captured)=>{const live=new Map(current.elements.map(value=>[value[0],value]));const paths=new Set(captured.elements.map(value=>value[0]));return{window:current.window,elements:[...captured.elements.map(([path,left])=>[path,left,live.get(path)?.[2]??0]),...current.elements.filter(([path])=>!paths.has(path))]}};\nconst capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;",
    );
    let output = output.replace(
        "const statefulStates=",
        &format!("const focusedTargets=[{focused_targets}];\nconst statefulStates="),
    );
    let output = output.replace(
        "const target=document.querySelector('[data-recreate-surface]:is(input,button,[tabindex]),[data-recreate-surface] input,[data-recreate-surface] button,[data-recreate-surface] [tabindex]')||surface;",
        "const target=(focusedTargets[state]&&document.querySelector(focusedTargets[state]))||document.querySelector('[data-recreate-surface]:is(input,button,[tabindex]),[data-recreate-surface] input,[data-recreate-surface] button,[data-recreate-surface] [tabindex]')||surface;",
    );
    let output = output.replace(
        "const carouselState=",
        &format!(
            "const initialScrolls={initial_scrolls};\nconst carouselPrevious={};\nconst carouselNext={};\nconst carouselState=",
            serde_json::to_string(&carousel_previous).unwrap(),
            serde_json::to_string(&carousel_next).unwrap()
        ),
    );
    let output = output.replace(
        "const lastTrigger=useRef('');const scroll=useRef(null);",
        "const lastTrigger=useRef('');const lastTriggerElement=useRef(null);const restoreFocus=useRef(null);const scroll=useRef(null);",
    );
    let output = output.replace(
        "const reset=()=>{const selector='[data-recreate-trigger=\"'+lastTrigger.current+'\"]';scroll.current=captureScroll(document.querySelector(selector));setState(0);requestAnimationFrame(()=>document.querySelector(selector)?.focus({preventScroll:true}))};",
        "const reset=()=>{const trigger=lastTriggerElement.current;scroll.current=captureScroll(trigger);restoreFocus.current=trigger;setState(0)};",
    );
    let output = output.replace(
        "const activate=(event,next,inputActive)=>{if(inputActive===false){if(state===next)reset();return}if(inputActive===true&&state===next)return;lastTrigger.current=event.currentTarget.dataset.recreateTrigger;",
        "const activate=(event,next,inputActive)=>{if(inputActive===false){if(state===next)reset();return}if(inputActive===true&&state===next)return;const previousTrigger=lastTriggerElement.current;previousTrigger?.removeAttribute('data-recreate-active');lastTriggerElement.current=event.currentTarget;event.currentTarget.dataset.recreateActive='true';lastTrigger.current=event.currentTarget.dataset.recreateTrigger;const[,command]=reduceInteraction({openSurface:state||null,activeTrigger:previousTrigger},{type:'activate',trigger:event.currentTarget,surface:next,stateful:statefulStates[next],closable:closableStates[next]});",
    );
    let output = output
        .replace("if(!statefulStates[next]){", "if(command.type==='invoke'){")
        .replace(
            "if(state===next){reset();return}",
            "if(command.type==='close'){reset();return}",
        )
        .replace(
            "captureScroll(event.currentTarget);setState(next)};useLayoutEffect",
            "captureScroll(event.currentTarget);setState(command.surface)};useLayoutEffect(()=>{for(const[path,left,top]of initialScrolls[viewport]||[])document.querySelector(path)?.scrollTo(left,top)},[viewport]);useLayoutEffect(()=>{if(state!==0)return;const trigger=restoreFocus.current;if(!trigger)return;const frame=requestAnimationFrame(()=>requestAnimationFrame(()=>{trigger.focus({preventScroll:true});trigger.removeAttribute('data-recreate-active');restoreFocus.current=null}));return()=>cancelAnimationFrame(frame)},[state]);useLayoutEffect",
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
    let children = components.children.get(path).cloned().unwrap_or_default();
    if let Some(extent) = leading_placeholder_extent(path, &children, components) {
        output.push_str(&leading_placeholder(depth, extent));
    }
    let mut previous_text = None;
    for child in &children {
        let node = &components.nodes[child];
        if previous_text.is_some_and(|previous| needs_text_space(previous, node)) {
            output.push_str(&format!("{}{{\" \"}}\n", "  ".repeat(depth)));
        }
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
        previous_text = (node.tag == "#text").then_some(node);
    }
    output
}

fn needs_text_space(previous: &crate::model::Node, current: &crate::model::Node) -> bool {
    if current.tag != "#text"
        || previous
            .text
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
        || current.text.chars().next().is_some_and(char::is_whitespace)
    {
        return false;
    }
    let same_line = (previous.rect.y - current.rect.y).abs() <= 1.0;
    let gap = current.rect.x - (previous.rect.x + previous.rect.width);
    same_line && gap > 0.5 && gap <= 12.0
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

#[cfg(test)]
#[path = "jsx_tests.rs"]
mod tests;
