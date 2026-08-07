pub const PROPERTIES: &str = concat!(
    "'display','visibility','position','float','inset','top','right','bottom','left','box-sizing',",
    "'width','height','min-width','max-width','min-height','max-height',",
    "'margin','margin-top','margin-right','margin-bottom','margin-left',",
    "'padding','padding-top','padding-right','padding-bottom','padding-left',",
    "'gap','row-gap','column-gap','flex','flex-grow',",
    "'flex-shrink','flex-basis','flex-direction','flex-wrap','justify-content',",
    "'align-items','align-self','justify-self','order','grid-template-columns',",
    "'grid-template-rows','grid-auto-flow','grid-column-start','grid-column-end',",
    "'grid-row-start','grid-row-end','border-collapse','border-spacing','table-layout',",
    "'caption-side','empty-cells','overflow','overflow-x','overflow-y','scrollbar-width',",
    "'scrollbar-gutter','scrollbar-color',",
    "'z-index','color','background-color','background-image','background-size',",
    "'background-position','background-repeat','background-clip','background-origin',",
    "'background-blend-mode','-webkit-background-clip','-webkit-text-fill-color',",
    "'border','border-radius','fill','stroke','stroke-width',",
    "'box-shadow','opacity','filter','transform','transform-origin',",
    "'font-family','font-size','font-weight','font-style','line-height',",
    "'font-stretch','font-kerning','font-feature-settings','font-variation-settings',",
    "'letter-spacing','text-align','vertical-align','text-transform','text-rendering',",
    "'white-space','word-break',",
    "'object-fit','object-position','cursor','pointer-events','transition',",
    "'animation','mask-image','mask-size','mask-position','mask-repeat',",
    "'mask-composite','clip-path',",
    // A property whose user-agent default differs from its CSS initial value cannot be
    // left uncaptured: the recreation renders the same tag, so the user-agent default
    // paints instead of the source's authored value. `textarea` is `resize: both` in
    // every UA stylesheet while the initial value is `none`, which draws a resize grip
    // the source does not have; links carry `text-decoration: underline`; and form
    // controls carry `appearance: auto`.
    "'resize','appearance','text-decoration-line','text-decoration-color',",
    "'text-decoration-style','text-decoration-thickness','text-overflow',",
    // `writing-mode` decides whether an authored `inline-size` means `width` or
    // `height`. It is read when mapping logical properties to physical ones, so
    // leaving it uncaptured made that mapping assume horizontal for every page.
    "'writing-mode'"
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
