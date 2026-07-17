use crate::model::PageState;

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
         @media(prefers-reduced-motion:reduce){.recreateStartupOverlay{animation:none!important;\
         display:none!important;opacity:0!important;visibility:hidden!important;pointer-events:none!important}}\
         .recreateStartupBlocking{position:fixed!important;inset:0!important;width:100vw!important;\
         height:100vh!important;max-width:100vw!important;max-height:100vh!important;\
         overflow:hidden!important}.recreateStartupBody{overflow:visible!important}\n",
    );
}

pub fn runtime(source: String, states: &[PageState]) -> String {
    let durations = states
        .iter()
        .map(|state| state.startup_duration_ms.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let source = source.replace(
        "const[state,setState]=useState(0);",
        "const[state,setState]=useState(0);const[startupDone,setStartupDone]=useState(false);",
    );
    let source = source.replace(
        "const View=baselineViews[viewport];const activate=",
        &format!(
            "const View=baselineViews[viewport];const startupDurations=[{durations}];\
             useLayoutEffect(()=>{{const duration=startupDurations[viewport];\
             if(startupDone||!duration)return;if(matchMedia('(prefers-reduced-motion: reduce)').matches){{\
             setStartupDone(true);return}}document.body.classList.add('recreateStartupBody');\
             const timer=setTimeout(()=>{{document.body.classList.remove('recreateStartupBody');\
             setStartupDone(true)}},duration+500);return()=>{{clearTimeout(timer);\
             document.body.classList.remove('recreateStartupBody')}}}},[viewport,startupDone]);\
             const activate="
        ),
    );
    source.replace(
        "return <View activate={activate}/>",
        "return <View activate={activate} showStartup={!startupDone} \
         onStartupDone={()=>setStartupDone(true)}/>",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Node, PageState, Rect, Viewport};

    #[test]
    fn marks_captured_startup_root() {
        let root = Node {
            path: "startup".into(),
            parent: None,
            tag: "div".into(),
            text: String::new(),
            attributes: Default::default(),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
            style: Default::default(),
            before: None,
            after: None,
        };
        let state = PageState {
            url: String::new(),
            title: String::new(),
            viewport: Viewport::default(),
            nodes: Vec::new(),
            startup_nodes: vec![root],
            startup_delay_ms: 100,
            startup_duration_ms: 500,
            animations: Vec::new(),
            state_styles: Vec::new(),
            attribute_sequences: Vec::new(),
            css_rules: Vec::new(),
            asset_urls: Vec::new(),
            asset_data: Default::default(),
        };
        let mut css = String::new();
        append(std::slice::from_ref(&state), &mut css);
        assert!(css.contains("--recreate-startup-duration"));
        assert!(
            runtime(
                "const View=baselineViews[viewport];const activate=".into(),
                &[]
            )
            .contains("startupDone")
        );
    }
}
