use super::startup_replay;
use crate::model::PageState;

/// The layer is hidden until its animation runs, which is what makes the reduced-motion case
/// need no rule of its own: `interactions::REDUCED_MOTION_CSS` already stops every animation,
/// and with the animation stopped the base rule's `opacity:0;visibility:hidden` is what
/// remains. Restating that here would emit a second `@media(prefers-reduced-motion:reduce)`
/// block saying what two rules already say.
pub fn append(states: &[PageState], css: &mut String) {
    if !states
        .iter()
        .any(|state| state.startup_nodes.iter().any(|node| node.parent.is_none()))
    {
        return;
    }
    css.push_str(
        "@keyframes recreateStartupOverlay{0%,94%{opacity:1;visibility:visible;\
         pointer-events:auto}100%{opacity:0;visibility:hidden;pointer-events:none}}\
         .recreateStartupOverlay{opacity:0;visibility:hidden;pointer-events:none;\
         animation-name:recreateStartupOverlay!important;animation-timing-function:linear!important;\
         animation-duration:var(--recreate-startup-duration,1ms)!important;\
         animation-delay:var(--recreate-startup-delay,0ms)!important;\
         animation-iteration-count:1!important;animation-direction:normal!important;\
         animation-play-state:running!important;\
         animation-fill-mode:forwards!important}\
         .recreateStartupBlocking{position:fixed!important;\
         left:var(--recreate-startup-x,0)!important;top:var(--recreate-startup-y,0)!important;\
         width:var(--recreate-startup-width,100vw)!important;\
         height:var(--recreate-startup-height,100vh)!important;\
         overflow:hidden!important}.recreateStartupBody{overflow:visible!important}\n",
    );
}

pub fn runtime(source: String, states: &[PageState]) -> String {
    let settles = states
        .iter()
        .map(|state| startup_replay::Replay::of(state).settle_ms().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = source.replace(
        "const[state,setState]=useState(0);",
        "const[state,setState]=useState(0);const[startupDone,setStartupDone]=useState(false);",
    );
    let source = source.replace(
        "const activate=",
        &format!(
            "const startupSettles=[{settles}];\
             useLayoutEffect(()=>{{const settle=startupSettles[viewport];\
             if(startupDone)return;if(!settle){{setStartupDone(true);return}}\
             if(matchMedia('(prefers-reduced-motion: reduce)').matches){{\
             setStartupDone(true);return}}document.body.classList.add('recreateStartupBody');\
             const timer=setTimeout(()=>{{document.body.classList.remove('recreateStartupBody');\
             setStartupDone(true)}},settle);return()=>{{clearTimeout(timer);\
             document.body.classList.remove('recreateStartupBody')}}}},[viewport,startupDone]);\
             const activate="
        ),
    );
    source.replace(
        "<View activate={activate}/>",
        "<View activate={activate} showStartup={!startupDone} \
         onStartupDone={()=>setStartupDone(true)}/>",
    )
}

#[cfg(test)]
#[path = "startup_overlays_tests.rs"]
mod tests;
