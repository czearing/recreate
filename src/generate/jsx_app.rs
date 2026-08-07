use super::{
    attribute_sequences, interaction_scroll, interactions, jsx_variants, startup_overlays,
    structural_tree, tree::Components,
};
use crate::model::Specification;
use std::collections::BTreeMap;

/// The app module is authored as real JavaScript under `templates/`, so a syntax
/// error is caught by `node --check` rather than by the browser gate. Every
/// interpolation point is a token that parses in place: `"__RECREATE_X__"` where a
/// value is expected, `//__RECREATE_X__` on its own line where a statement is.
const APP_TEMPLATE: &str = concat!(
    include_str!("templates/app_runtime.mjs"),
    include_str!("templates/app_component.jsx"),
);

/// Substitute placeholder tokens into an extracted template.
///
/// `format!` substitutes simultaneously; this substitutes in sequence, so the two
/// agree only while no replacement value contains a token. The debug assertions
/// hold that invariant and catch a token that never matched anything.
pub(super) fn fill(template: &str, holes: &[(&str, &str)]) -> String {
    let mut output = template.to_string();
    for (token, value) in holes {
        debug_assert!(
            output.contains(token),
            "template has no placeholder {token}"
        );
        debug_assert!(
            !value.contains("__RECREATE_"),
            "value for {token} would re-introduce a placeholder"
        );
        output = output.replace(token, value);
    }
    output
}

/// Splice snippets are stored with a trailing newline so they are ordinary text
/// files; that newline is never part of the emitted JavaScript.
fn snippet(text: &str) -> &str {
    text.trim_end_matches('\n')
}

pub fn app(
    specification: &Specification,
    components: &Components,
    class_maps: &[BTreeMap<String, String>],
    assets: &BTreeMap<String, String>,
) -> String {
    if specification.states.is_empty() {
        return "export default function App(){return null}\n".into();
    }
    let bodies = specification
        .states
        .iter()
        .zip(class_maps)
        .map(|(state, classes)| {
            let mut handlers = interactions::base_handlers(specification, state);
            attribute_sequences::append_handlers(state, &mut handlers);
            let current = structural_tree::for_state(components, state, classes);
            let page = jsx_variants::page(state, &current, assets, &handlers);
            if state.startup_nodes.is_empty() {
                return page;
            }
            let startup = structural_tree::fragment_nodes(&state.startup_nodes, classes);
            let fragment = jsx_variants::fragment(
                &startup,
                assets,
                state.startup_delay_ms,
                state.startup_duration_ms,
            );
            format!("<>{page}{{createPortal({fragment},document.body)}}</>")
        })
        .collect::<Vec<_>>();
    let mut unique_bodies: Vec<&String> = Vec::new();
    let view_indexes = bodies
        .iter()
        .map(|body| {
            unique_bodies
                .iter()
                .position(|existing| *existing == body)
                .unwrap_or_else(|| {
                    unique_bodies.push(body);
                    unique_bodies.len() - 1
                })
        })
        .collect::<Vec<_>>();
    let views = unique_bodies
        .iter()
        .enumerate()
        .map(|(index, body)| {
            format!(
                "function Baseline{index}({{activate,showStartup,onStartupDone}}){{return {body}}}\n"
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
    let view_names = view_indexes
        .iter()
        .map(|index| format!("Baseline{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let widths = jsx_variants::widths(&specification.states);
    let state_imports = (1..=specification.interactions.len())
        .map(|index| format!("Interaction{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let component_names = components
        .items
        .iter()
        .map(|item| item.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let component_import = if component_names.is_empty() {
        String::new()
    } else {
        format!("import {{ {component_names} }} from './components/index.js';\n")
    };
    let state_import = if state_imports.is_empty() {
        String::new()
    } else {
        format!("import {{ {state_imports} }} from './states.jsx';\n")
    };
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
    let responsive_attribute_paths = responsive_attributes.paths;
    let responsive_attribute_values = responsive_attributes.values;
    let responsive_attributes = responsive_attributes.viewports;
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
    let selector = jsx_variants::selector();
    let carousel_state = carousel_state.to_string();
    let output = fill(
        APP_TEMPLATE,
        &[
            ("//__RECREATE_COMPONENT_IMPORT__\n", &component_import),
            ("//__RECREATE_STATE_IMPORT__\n", &state_import),
            ("\"__RECREATE_POSITIONAL__\"", selector),
            ("\"__RECREATE_WIDTHS__\"", &widths),
            ("\"__RECREATE_CLOSABLE__\"", &closable),
            ("\"__RECREATE_STATEFUL__\"", &stateful),
            ("\"__RECREATE_REPLACEMENT_STATES__\"", &replacement_states),
            ("\"__RECREATE_SCROLL_TARGETS__\"", &scroll_targets),
            ("\"__RECREATE_CAROUSEL_STATE__\"", &carousel_state),
            ("\"__RECREATE_ATTRIBUTE_SEQUENCES__\"", &attribute_sequences),
            (
                "\"__RECREATE_RESPONSIVE_ATTRIBUTE_PATHS__\"",
                &responsive_attribute_paths,
            ),
            (
                "\"__RECREATE_RESPONSIVE_ATTRIBUTE_VALUES__\"",
                &responsive_attribute_values,
            ),
            (
                "\"__RECREATE_RESPONSIVE_ATTRIBUTES__\"",
                &responsive_attributes,
            ),
            ("//__RECREATE_VIEWS__\n", &views),
            ("\"__RECREATE_VIEW_NAMES__\"", &view_names),
            ("\"__RECREATE_RENDER_STATE__\"", &render_state),
        ],
    );
    let output = output.replace(
        "const setScroll=(element,left,top)=>element===window?scrollTo(left,top):element.scrollTo(left,top);",
        snippet(include_str!("templates/scroll_extent.mjs")),
    );
    let output = output.replace(
        "document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>element.setAttribute('aria-expanded',String(Number(element.dataset.recreateTrigger)===state)));",
        "document.querySelectorAll('[data-recreate-trigger][aria-expanded]').forEach(element=>{const expanded=transitionGraph?transitionEdges.find(edge=>edge.from===0&&edge.key===element.dataset.recreateTrigger)?.to===state:Number(element.dataset.recreateTrigger)===state;element.setAttribute('aria-expanded',String(expanded))});",
    );
    let output = output.replace(
        "if(state&&closableStates[state])requestAnimationFrame",
        snippet(include_str!("templates/sync_active_controls.mjs")),
    );
    let output = output.replace(
        "const capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;",
        snippet(include_str!("templates/merge_scroll.mjs")),
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
        snippet(include_str!("templates/activate_transition.fragment")),
    );
    let output = output
        .replace("if(!statefulStates[next]){", "if(command.type==='invoke'){")
        .replace(
            "if(state===next){reset();return}",
            "if(command.type==='close'){reset();return}",
        )
        .replace(
            "captureScroll(event.currentTarget);setState(next)};useLayoutEffect",
            snippet(include_str!("templates/state_effects.fragment")),
        );
    let output = output.replace(
        "scroll.current=event.currentTarget.dataset.recreatePreserveScroll==='false'?(captured??{window:[0,0],elements:[]}):captureScroll(event.currentTarget);setState(command.surface)",
        snippet(include_str!("templates/tab_scroll.mjs")),
    );
    let output = attribute_sequences::runtime(output);
    let output = output.replace(
        "useEffect(()=>startSequences(document,attributeSequences[viewport]||[]),[viewport,state]);",
        snippet(include_str!("templates/transition_dispatch.mjs")),
    );
    let output = startup_overlays::runtime(output, &specification.states);
    output.replace(
        "const[state,setState]=useState(0);",
        "const[state,setState]=useState(()=>{const saved=Number(sessionStorage.getItem(returnStorageKey)||0);sessionStorage.removeItem(returnStorageKey);return saved});",
    )
}
