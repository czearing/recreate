pub const PROPERTIES: &str = concat!(
    "'display','position','inset','top','right','bottom','left','box-sizing',",
    "'width','height','min-width','max-width','min-height','max-height',",
    "'margin','padding','gap','row-gap','column-gap','flex','flex-grow',",
    "'flex-shrink','flex-basis','flex-direction','flex-wrap','justify-content',",
    "'align-items','align-self','justify-self','order','grid-template-columns',",
    "'grid-template-rows','grid-auto-flow','grid-column-start','grid-column-end',",
    "'grid-row-start','grid-row-end','overflow','overflow-x','overflow-y',",
    "'z-index','color','background-color','background-image','background-size',",
    "'background-position','background-repeat','border','border-radius',",
    "'box-shadow','opacity','filter','transform','transform-origin',",
    "'font-family','font-size','font-weight','font-style','line-height',",
    "'font-stretch','font-kerning','font-feature-settings','font-variation-settings',",
    "'letter-spacing','text-align','text-transform','text-rendering','white-space','word-break',",
    "'object-fit','object-position','cursor','pointer-events','transition',",
    "'animation','mask-image','mask-size','mask-position','mask-repeat',",
    "'mask-composite','clip-path'"
);

pub const DIRECTIONAL_BORDERS: &str = concat!(
    "'border-top-width','border-right-width','border-bottom-width','border-left-width',",
    "'border-top-style','border-right-style','border-bottom-style','border-left-style',",
    "'border-top-color','border-right-color','border-bottom-color','border-left-color'"
);

#[cfg(test)]
pub fn contains(name: &str) -> bool {
    PROPERTIES.contains(&format!("'{name}'")) || DIRECTIONAL_BORDERS.contains(&format!("'{name}'"))
}
