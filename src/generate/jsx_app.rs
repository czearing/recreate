use super::{
    attribute_sequences, interaction_scroll, interactions, jsx_variants, startup_overlays,
    structural_tree, tree::Components,
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
                format!("{{createPortal({fragment},document.body)}}")
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
    let render_state = state_overlay.replace("state===", "value===");
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
    let carousel_controls = carousel.and_then(|(_, interaction)| {
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
            })?;
        Some((previous.path.clone(), trigger.path.clone()))
    });
    let carousel_state = carousel_controls
        .as_ref()
        .and_then(|(_, next)| {
            carousel
                .filter(|(_, interaction)| interaction.trigger_path == *next)
                .map(|(index, _)| index + 1)
        })
        .unwrap_or_default();
    let (carousel_previous, carousel_next) = carousel_controls.unwrap_or_default();
    let attribute_sequences = attribute_sequences::javascript(specification);
    let responsive_attributes =
        super::responsive_attributes::javascript(&specification.states, canonical);
    let initial_scrolls = super::initial_scroll::targets(specification);
    let inferred_carousel =
        super::carousel_inference::javascript(specification, carousel_state != 0);
    let transition_edges = serde_json::to_string(
        &specification
            .transitions
            .iter()
            .map(|transition| {
                serde_json::json!({
                    "from":transition.from_state,
                    "to":transition.to_state,
                    "key":interactions::transition_key(transition),
                    "action":transition.action
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let baseline_selected = specification.transitions.iter().find(|transition| {
        specification.states.first().is_some_and(|baseline| {
            baseline
                .nodes
                .iter()
                .find(|node| node.path == transition.trigger_path)
                .and_then(|node| node.attributes.get("aria-selected"))
                .is_some_and(|value| value == "true")
        })
    });
    let baseline_selected_tokens = serde_json::to_string(
        &baseline_selected
            .into_iter()
            .map(interactions::transition_key)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let baseline_selected_state = baseline_selected.map_or(0, |transition| transition.to_state);
    let control_styles = serde_json::to_string(
        &specification
            .transitions
            .iter()
            .filter(|transition| transition.action == crate::model::InteractionAction::Activate)
            .filter_map(|transition| {
                let baseline = specification.states.first()?;
                let baseline_node = baseline
                    .nodes
                    .iter()
                    .find(|node| node.path == transition.trigger_path)?;
                if !baseline_node.attributes.contains_key("aria-selected")
                    && !baseline_node.attributes.contains_key("aria-pressed")
                {
                    return None;
                }
                let state = transition
                    .to_state
                    .checked_sub(1)
                    .and_then(|index| specification.interactions.get(index))?
                    .states
                    .first()?;
                let active_node = state
                    .nodes
                    .iter()
                    .find(|node| node.path == transition.trigger_path)?;
                let state_attribute = if baseline_node.attributes.contains_key("aria-selected") {
                    "aria-selected"
                } else {
                    "aria-pressed"
                };
                let inactive_node = baseline
                    .nodes
                    .iter()
                    .find(|node| {
                        node.parent == baseline_node.parent
                            && node
                                .attributes
                                .get(state_attribute)
                                .is_some_and(|value| value == "false")
                    })
                    .unwrap_or(baseline_node);
                let properties = [
                    "background-color",
                    "border-color",
                    "box-shadow",
                    "color",
                    "filter",
                    "opacity",
                    "outline-color",
                ];
                let styles = |node: &crate::model::Node| {
                    properties
                        .iter()
                        .filter_map(|name| {
                            node.style
                                .get(*name)
                                .map(|value| ((*name).to_string(), value.clone()))
                        })
                        .collect::<BTreeMap<_, _>>()
                };
                let foreground = |state: &crate::model::PageState| {
                    state
                        .nodes
                        .iter()
                        .find(|node| {
                            node.path
                                .starts_with(&format!("{}>", transition.trigger_path))
                                && node.tag == "span"
                                && state.nodes.iter().any(|child| {
                                    child.tag == "#text"
                                        && child.path.starts_with(&format!("{}>", node.path))
                                        && !child.text.trim().is_empty()
                                })
                        })
                        .map(|node| {
                            ["-webkit-text-fill-color", "color", "opacity"]
                                .iter()
                                .filter_map(|name| {
                                    node.style
                                        .get(*name)
                                        .map(|value| ((*name).to_string(), value.clone()))
                                })
                                .collect::<BTreeMap<_, _>>()
                        })
                        .unwrap_or_default()
                };
                Some((
                    interactions::transition_key(transition),
                    serde_json::json!({
                        "active": styles(active_node),
                        "inactive": styles(inactive_node),
                        "activeForeground": foreground(state),
                        "inactiveForeground": foreground(baseline)
                    }),
                ))
            })
            .collect::<BTreeMap<_, _>>(),
    )
    .unwrap();
    let baseline_pressed_tokens = serde_json::to_string(
        &specification
            .transitions
            .iter()
            .filter(|transition| {
                specification.states.first().is_some_and(|baseline| {
                    baseline
                        .nodes
                        .iter()
                        .find(|node| node.path == transition.trigger_path)
                        .and_then(|node| node.attributes.get("aria-pressed"))
                        .is_some_and(|value| value == "true")
                })
            })
            .map(interactions::transition_key)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let output = format!(
        "import React,{{useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore}} from 'react';\nimport {{createPortal}} from 'react-dom';\nimport {{moveCarousel}} from './runtime/carousel.mjs';\nimport {{reduceInteraction}} from './runtime/interaction.mjs';\nimport {{startSequences}} from './runtime/sequence.mjs';\nimport {{ {} }} from './components/index.js';\nimport {{ {} }} from './states.jsx';\nconst keyActivate=(event,action)=>{{if(event.key==='Enter'||event.key===' '){{event.preventDefault();action(event)}}}};\nconst pathOf=element=>{{const parts=[];for(let node=element;node&&node!==document.documentElement;node=node.parentElement){{const peers=node.parentElement?[...node.parentElement.children].filter(child=>child.tagName===node.tagName):[node];parts.push(`${{node.tagName.toLowerCase()}}:nth-of-type(${{peers.indexOf(node)+1}})`)}}return `html>${{parts.reverse().join('>')}}`}};\nconst captureScroll=element=>{{const elements=[];for(let node=element?.parentElement;node&&node!==document.documentElement;node=node.parentElement){{if(node.scrollLeft||node.scrollTop)elements.push([pathOf(node),node.scrollLeft,node.scrollTop])}}return{{window:[scrollX,scrollY],elements}}}};\n        const scrollAnimations=new WeakMap();const scrollEase=value=>{{let current=value;for(let index=0;index<5;index++){{const inverse=1-current;const x=3*inverse*inverse*current*.4+3*inverse*current*current*.2+current*current*current;const slope=3*inverse*inverse*.4+6*inverse*current*(.2-.4)+3*current*current*(1-.2);if(Math.abs(slope)<1e-4)break;current=Math.max(0,Math.min(1,current-(x-value)/slope))}}const inverse=1-current;return 3*inverse*current*current+current*current*current}};const setScroll=(element,left,top)=>element===window?scrollTo(left,top):element.scrollTo(left,top);const animateScroll=(element,left,top)=>{{if(element!==window&&top===0){{const content=[...element.children].find(child=>child.scrollWidth>element.clientWidth&&getComputedStyle(child).transition.includes('transform'));        if(content){{element.scrollTo(0,0);requestAnimationFrame(()=>requestAnimationFrame(()=>{{content.style.transform=`translateX(${{-left}}px)`}}));return}}}}const startLeft=element===window?scrollX:element.scrollLeft;const startTop=element===window?scrollY:element.scrollTop;if(Math.abs(startLeft-left)<1&&Math.abs(startTop-top)<1)return;const token={{}};scrollAnimations.set(element,token);const started=performance.now();const frame=now=>{{if(scrollAnimations.get(element)!==token)return;const progress=Math.min(1,(now-started)/320);const eased=scrollEase(progress);setScroll(element,startLeft+(left-startLeft)*eased,startTop+(top-startTop)*eased);if(progress<1)requestAnimationFrame(frame)}};requestAnimationFrame(frame)}};const restoreScroll=snapshot=>{{if(snapshot.smooth){{animateScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{{const element=document.querySelector(path);if(element)animateScroll(element,left,top)}});return}}setScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{{const element=document.querySelector(path);if(element)setScroll(element,left,top)}})}};\n{}\nconst viewportWidths=[{widths}];\nconst closableStates=[{closable}];\nconst statefulStates=[{stateful}];\nconst replacementStates=[{replacement_states}];\nconst capturedScrolls={scroll_targets};\nconst carouselState={carousel_state};\nconst attributeSequences={attribute_sequences};\nconst responsiveAttributes={responsive_attributes};\nconst capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;\n        const subscribe=notify=>{{const media=viewportWidths.slice(1).map(width=>matchMedia(`(max-width:${{width}}px)`));media.forEach(query=>query.addEventListener('change',notify));addEventListener('resize',notify);return()=>{{media.forEach(query=>query.removeEventListener('change',notify));removeEventListener('resize',notify)}}}};\n{views}const baselineViews=[{view_names}];\n                                                export default function App(){{const[state,setState]=useState(0);const[scrollRevision,setScrollRevision]=useState(0);const[carouselAdvanced,setCarouselAdvanced]=useState(false);const lastTrigger=useRef('');const scroll=useRef(null);const width=useSyncExternalStore(subscribe,()=>document.documentElement.clientWidth,()=>0);const viewport=selectViewport(width,viewportWidths);const reset=()=>{{const selector='[data-recreate-trigger=\"'+lastTrigger.current+'\"]';scroll.current=captureScroll(document.querySelector(selector));setState(0);requestAnimationFrame(()=>document.querySelector(selector)?.focus({{preventScroll:true}}))}};const activate=(event,next,inputActive)=>{{if(inputActive===false){{if(state===next)reset();return}}if(inputActive===true&&state===next)return;lastTrigger.current=event.currentTarget.dataset.recreateTrigger;const captured=capturedScroll(next,viewport);if(!statefulStates[next]){{scroll.current=captured?{{...mergeHorizontalScroll(captureScroll(event.currentTarget),captured),smooth:true}}:null;if(next===carouselState)setCarouselAdvanced(true);if(scroll.current)setScrollRevision(value=>value+1);return}}if(state===next){{reset();return}}scroll.current=event.currentTarget.dataset.recreatePreserveScroll==='false'?(captured??{{window:[0,0],elements:[]}}):captureScroll(event.currentTarget);setState(next)}};useLayoutEffect(()=>{{if(state!==0)return;for(const[path,attributes]of responsiveAttributes[viewport]||[]){{const element=document.querySelector(path);if(!element)continue;for(const[name,value]of attributes)value===null?element.removeAttribute(name):element.setAttribute(name,value)}}}},[viewport,state]);useLayoutEffect(()=>{{document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>element.setAttribute('aria-expanded',String(Number(element.dataset.recreateTrigger)===state)));if(state&&closableStates[state])requestAnimationFrame(()=>{{const surfaces=[...document.querySelectorAll('[data-recreate-surface]')];const surface=surfaces.at(-1);const target=(focusedTargets[state]&&document.querySelector(focusedTargets[state]))||[...surfaces].reverse().find(element=>element.matches('input,button,[tabindex]'))||surface?.querySelector('input,button,[tabindex]')||surface;if(target){{if(target===surface&&!target.matches('input,button,[tabindex]'))target.tabIndex=-1;target.focus({{preventScroll:true}})}}}});if(!scroll.current)return;const snapshot=scroll.current;restoreScroll(snapshot);if(!snapshot.smooth)requestAnimationFrame(()=>restoreScroll(snapshot))}},[state,scrollRevision]);useEffect(()=>{{if(!carouselState||!carouselPrevious||!carouselNext)return;const previous=document.querySelector(carouselPrevious);const more=document.querySelector(carouselNext);if(!previous||!more)return;previous.disabled=!carouselAdvanced;more.disabled=carouselAdvanced;const reverse=()=>{{if(!carouselAdvanced)return;const captured=capturedScroll(carouselState,viewport);if(!captured)return;const origin={{window:[0,0],elements:captured.elements.map(([path,left,top])=>[path,0,top])}};scroll.current={{...mergeHorizontalScroll(captureScroll(more),origin),smooth:true}};setCarouselAdvanced(false);setScrollRevision(value=>value+1)}};previous.addEventListener('click',reverse);return()=>previous.removeEventListener('click',reverse)}},[carouselAdvanced,viewport]);useEffect(()=>{{const timers=(attributeSequences[viewport]||[]).map((sequence,index)=>{{const element=document.querySelector(`[data-recreate-sequence=\"${{index}}\"]`);if(!element||sequence.values.length<2)return null;let current=0;element.setAttribute(sequence.attribute,sequence.values[current]);return setInterval(()=>{{current=(current+1)%sequence.values.length;element.setAttribute(sequence.attribute,sequence.values[current])}},sequence.interval_ms)}});return()=>timers.forEach(timer=>timer&&clearInterval(timer))}},[viewport]);useEffect(()=>{{if(!state||!closableStates[state])return;const key=event=>{{if(event.key==='Escape')reset()}};const pointer=event=>{{if(!event.target.closest('[data-recreate-surface],[data-recreate-control]'))reset()}}        ;addEventListener('keydown',key);addEventListener('pointerdown',pointer);return()=>{{removeEventListener('keydown',key);removeEventListener('pointerdown',pointer)}}}},[state]);const renderState=value=>{render_state};const contentState=closableStates[state]?returnState.current:state;const content=renderState(contentState);const popup=closableStates[state]?renderState(state):null;const baseline=baselineViews[viewport]({{activate,showStartup:!startupDone,onStartupDone:()=>setStartupDone(true)}});return replacementStates[contentState]?<>{{content}}{{popup}}</>:<>{{baseline}}{{content}}{{popup}}</>}}\n",
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
        "const setScroll=(element,left,top)=>element===window?scrollTo(left,top):element.scrollTo(left,top);",
        "const ensureScrollExtent=(element,left)=>{if(element===window||left<=element.scrollWidth-element.clientWidth)return;let extent=[...element.children].find(child=>child.hasAttribute('data-recreate-scroll-extent'));if(!extent){extent=document.createElement('span');extent.setAttribute('data-recreate-startup','true');extent.setAttribute('data-recreate-scroll-extent','true');extent.style.cssText='display:block;height:0;flex:none;pointer-events:none;visibility:hidden';element.append(extent)}extent.style.width=`${element.clientWidth+left}px`};const setScroll=(element,left,top)=>{ensureScrollExtent(element,left);element===window?scrollTo(left,top):element.scrollTo(left,top)};",
    );
    let output = output.replace(
        "document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>element.setAttribute('aria-expanded',String(Number(element.dataset.recreateTrigger)===state)));",
        "document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>{const expanded=transitionGraph?transitionEdges.find(edge=>edge.from===0&&edge.key===element.dataset.recreateTrigger)?.to===state:Number(element.dataset.recreateTrigger)===state;element.setAttribute('aria-expanded',String(expanded))});",
    );
    let output = output.replace(
        "if(state&&closableStates[state])requestAnimationFrame",
        "if(transitionGraph){const syncActive=()=>{const activeState=closableStates[state]?returnState.current:state;for(const attribute of ['aria-selected','aria-pressed']){const name=attribute==='aria-selected'?'recreateSelected':'recreatePressed';const baselineToken=(attribute==='aria-selected'?baselineSelectedTokens:baselinePressedTokens)[0];const remembered=document.documentElement.dataset[name]||baselineToken;document.querySelectorAll(`[data-recreate-trigger][${attribute}]`).forEach(element=>{if(!remembered&&activeState===0)return;const active=remembered?element.dataset.recreateTrigger===remembered:transitionEdges.find(edge=>edge.from===0&&edge.key===element.dataset.recreateTrigger)?.to===activeState;element.setAttribute(attribute,String(active));const styles=controlStyles[element.dataset.recreateTrigger]||{};const paint=styles[active?'active':'inactive']||{};for(const[property,value]of Object.entries(paint))element.style.setProperty(property,value,'important');const foreground=styles[active?'activeForeground':'inactiveForeground']||{};for(const child of element.querySelectorAll('span'))for(const[property,value]of Object.entries(foreground))child.style.setProperty(property,value,'important')})}};syncActive();requestAnimationFrame(syncActive)}if(state&&closableStates[state])requestAnimationFrame",
    );
    let output = output.replace(
        "const capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;",
        "const mergeHorizontalScroll=(current,captured)=>{const live=new Map(current.elements.map(value=>[value[0],value]));const paths=new Set(captured.elements.map(value=>value[0]));return{window:current.window,elements:[...captured.elements.map(([path,left])=>[path,left,live.get(path)?.[2]??0]),...current.elements.filter(([path])=>!paths.has(path))]}};\nconst mergeStateScroll=(current,captured)=>{const live=new Map(current.elements.map(value=>[value[0],value]));const paths=new Set(captured.elements.map(value=>value[0]));return{window:current.window,elements:[...captured.elements.map(([path,,top])=>[path,live.get(path)?.[1]??0,top]),...current.elements.filter(([path])=>!paths.has(path))]}};\nconst capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;",
    );
    let output = output.replace(
        "const statefulStates=",
        &format!("const focusedTargets=[{focused_targets}];\nconst statefulStates="),
    );
    let output = output.replace(
        "const viewportWidths=",
        &format!(
            "{inferred_carousel}\nconst transitionGraph={};\nconst transitionEdges={transition_edges};\nconst controlStyles={control_styles};\nconst baselineSelectedTokens={baseline_selected_tokens};\nconst baselineSelectedState={baseline_selected_state};\nconst baselinePressedTokens={baseline_pressed_tokens};\nconst returnStorageKey={};\nconst viewportWidths=",
            !specification.transitions.is_empty(),
            serde_json::to_string(&format!(
                "recreateReturnState:{}",
                specification.captured_url
            ))
            .unwrap()
        ),
    );
    let output = output.replace(
        "useEffect(()=>{if(!carouselState||!carouselPrevious||!carouselNext)return;",
        &format!(
            "{}useEffect(()=>{{if(!carouselState||!carouselPrevious||!carouselNext)return;",
            super::carousel_inference::EFFECT
        ),
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
        "const lastTrigger=useRef('');const lastTriggerElement=useRef(null);const restoreFocus=useRef(null);const returnState=useRef(0);const selectedState=useRef(baselineSelectedState);const scroll=useRef(null);",
    );
    let output = output.replace(
        "const reset=()=>{const selector='[data-recreate-trigger=\"'+lastTrigger.current+'\"]';scroll.current=captureScroll(document.querySelector(selector));setState(0);requestAnimationFrame(()=>document.querySelector(selector)?.focus({preventScroll:true}))};",
        "const reset=()=>{const trigger=lastTriggerElement.current;trigger?.removeAttribute('data-recreate-active');lastTrigger.current='';scroll.current=captureScroll(trigger);restoreFocus.current=trigger;if(transitionGraph&&document.querySelector('[data-recreate-surface]')){setState(returnState.current);return}setState(0)};",
    );
    let output = output.replace(
        "const activate=(event,next,inputActive)=>{if(inputActive===false){if(state===next)reset();return}if(inputActive===true&&state===next)return;lastTrigger.current=event.currentTarget.dataset.recreateTrigger;",
        "const activate=(event,next,inputActive,actionType='activate')=>{if(transitionGraph){const currentState=Number(document.documentElement.dataset.recreateState||state);const repeatedSurface=actionType==='activate'&&closableStates[currentState]&&event.currentTarget.dataset.recreateActive==='true'&&document.querySelector('[data-recreate-surface]');const openedBy=transitionEdges.some(candidate=>candidate.from===0&&candidate.to===currentState&&candidate.key===next&&candidate.action===actionType);if(repeatedSurface||currentState&&closableStates[currentState]&&(openedBy||lastTrigger.current===next))next=returnState.current;else{let matches=transitionEdges.filter(candidate=>candidate.from===currentState&&candidate.key===next&&candidate.action===actionType);if(!matches.length&&!closableStates[currentState])matches=transitionEdges.filter(candidate=>candidate.from===0&&candidate.key===next&&candidate.action===actionType);const edge=matches.find(candidate=>candidate.to!==currentState)??matches[0];if(!edge)return;next=edge.to}if(event.currentTarget.hasAttribute('aria-selected'))selectedState.current=next;if(event.currentTarget.hasAttribute('aria-pressed')&&baselinePressedTokens.includes(event.currentTarget.dataset.recreateTrigger))next=selectedState.current;if(closableStates[currentState]&&!closableStates[next]){const trigger=lastTriggerElement.current??event.currentTarget;trigger?.removeAttribute('data-recreate-active');lastTrigger.current='';restoreFocus.current=trigger;scroll.current=captureScroll(trigger);setState(next);return}if(event.currentTarget.hasAttribute('aria-selected'))document.documentElement.dataset.recreateSelected=event.currentTarget.dataset.recreateTrigger;if(event.currentTarget.hasAttribute('aria-pressed'))document.documentElement.dataset.recreatePressed=event.currentTarget.dataset.recreateTrigger;if(closableStates[next])returnState.current=currentState;const previousTrigger=lastTriggerElement.current;previousTrigger?.removeAttribute('data-recreate-active');lastTriggerElement.current=event.currentTarget;event.currentTarget.dataset.recreateActive='true';lastTrigger.current=next===0?'':event.currentTarget.dataset.recreateTrigger;const destination=capturedScroll(next,viewport);const origin=capturedScroll(currentState,viewport);const stateControl=event.currentTarget.getAttribute('role')==='tab'||event.currentTarget.hasAttribute('aria-selected')||event.currentTarget.hasAttribute('aria-pressed');const preservePosition=closableStates[currentState]||closableStates[next];scroll.current=stateControl&&destination?{...mergeStateScroll(captureScroll(event.currentTarget),destination),smooth:true}:preservePosition?captureScroll(event.currentTarget):destination??(next===0&&origin?{window:[0,0],elements:origin.elements.map(([path])=>[path,0,0])}:null);setState(next);return}if(inputActive===false){if(state===next)reset();return}if(inputActive===true&&state===next)return;const previousTrigger=lastTriggerElement.current;previousTrigger?.removeAttribute('data-recreate-active');lastTriggerElement.current=event.currentTarget;event.currentTarget.dataset.recreateActive='true';lastTrigger.current=event.currentTarget.dataset.recreateTrigger;const[,command]=reduceInteraction({openSurface:state||null,activeTrigger:previousTrigger},{type:'activate',trigger:event.currentTarget,surface:next,stateful:statefulStates[next],closable:closableStates[next]});",
    );
    let output = output
        .replace("if(!statefulStates[next]){", "if(command.type==='invoke'){")
        .replace(
            "if(state===next){reset();return}",
            "if(command.type==='close'){reset();return}",
        )
        .replace(
            "captureScroll(event.currentTarget);setState(next)};useLayoutEffect",
            "captureScroll(event.currentTarget);setState(command.surface)};useLayoutEffect(()=>{document.documentElement.dataset.recreateState=String(state);for(const name of ['Selected','Pressed']){const key=`${returnStorageKey}:${name}`;const value=sessionStorage.getItem(key);if(value)document.documentElement.dataset[`recreate${name}`]=value;sessionStorage.removeItem(key)}},[state]);useLayoutEffect(()=>{for(const[path,left,top]of initialScrolls[viewport]||[])document.querySelector(path)?.scrollTo(left,top)},[viewport]);useLayoutEffect(()=>{if(closableStates[state])return;const trigger=restoreFocus.current;if(!trigger)return;const frame=requestAnimationFrame(()=>requestAnimationFrame(()=>{trigger.focus({preventScroll:true});trigger.removeAttribute('data-recreate-active');restoreFocus.current=null}));return()=>cancelAnimationFrame(frame)},[state]);useLayoutEffect",
        );
    let output = output.replace(
        "scroll.current=event.currentTarget.dataset.recreatePreserveScroll==='false'?(captured??{window:[0,0],elements:[]}):captureScroll(event.currentTarget);setState(command.surface)",
        "scroll.current=event.currentTarget.getAttribute('role')==='tab'&&captured?{window:captured.window,elements:captured.elements.map(([path,,top],index)=>[path,index===0&&width<=390&&captured.elements.some(([,left])=>left>5)?5:0,top])}:event.currentTarget.dataset.recreatePreserveScroll==='false'?(captured??{window:[0,0],elements:[]}):(event.currentTarget.hasAttribute('aria-pressed')&&captured?captured:captureScroll(event.currentTarget));setState(command.surface)",
    );
    let output = attribute_sequences::runtime(output);
    let output = output.replace(
        "useEffect(()=>startSequences(document,attributeSequences[viewport]||[]),[viewport,state]);",
        "useLayoutEffect(()=>{if(!transitionGraph)return;const dispatch=(event,action)=>{const control=event.target.closest?.('[data-recreate-trigger]');if(!control)return;const token=control.dataset.recreateTrigger;const currentState=Number(document.documentElement.dataset.recreateState||0);const repeatedSurface=action==='activate'&&closableStates[currentState]&&control.dataset.recreateActive==='true'&&document.querySelector('[data-recreate-surface]');const direct=transitionEdges.some(edge=>edge.from===currentState&&edge.key===token&&edge.action===action);const baseline=!closableStates[currentState]&&transitionEdges.some(edge=>edge.from===0&&edge.key===token&&edge.action===action);if(!direct&&!baseline&&!repeatedSurface)return;activate({currentTarget:control},token,undefined,action)};const click=event=>dispatch(event,'activate');const hover=event=>dispatch(event,'hover');const leave=event=>{const control=event.target.closest?.('[data-recreate-trigger]');if(control?.contains(event.relatedTarget))return;dispatch(event,'leave')};const focus=event=>dispatch(event,'focus');const input=event=>dispatch(event,'activate');const key=event=>{if((event.key==='Enter'||event.key===' ')&&event.target.matches('button,[role=button],[role=tab],summary')){event.preventDefault();event.stopImmediatePropagation();dispatch(event,'activate')}};addEventListener('click',click,true);addEventListener('pointerover',hover,true);addEventListener('pointerout',leave,true);addEventListener('focusin',focus,true);addEventListener('input',input,true);addEventListener('keydown',key,true);return()=>{removeEventListener('click',click,true);removeEventListener('pointerover',hover,true);removeEventListener('pointerout',leave,true);removeEventListener('focusin',focus,true);removeEventListener('input',input,true);removeEventListener('keydown',key,true)}},[]);useEffect(()=>startSequences(document,attributeSequences[viewport]||[]),[viewport,state]);",
    );
    let output = startup_overlays::runtime(output, &specification.states);
    output.replace(
        "const[state,setState]=useState(0);",
        "const[state,setState]=useState(()=>{const saved=Number(sessionStorage.getItem(returnStorageKey)||0);sessionStorage.removeItem(returnStorageKey);return saved});",
    )
}
