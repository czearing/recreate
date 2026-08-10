//! The name tables `jsx_attr_names` reads in both directions.
//!
//! They live beside the rule rather than inside it only to keep each file readable. There
//! is still exactly one copy of each list, which is what stops the JSX spelling and the
//! document spelling of a name from drifting apart.
/// React's canonical prop names for HTML attributes that differ from their HTML spelling
/// only by case. Taken from the React DOM attribute reference.
pub(super) const CAMEL_CASED: &[&str] = &[
    "acceptCharset",
    "accessKey",
    "allowFullScreen",
    "autoCapitalize",
    "autoComplete",
    "autoCorrect",
    "autoFocus",
    "autoPlay",
    "autoSave",
    "cellPadding",
    "cellSpacing",
    "charSet",
    "classID",
    "colSpan",
    "contentEditable",
    "contextMenu",
    "controlsList",
    "crossOrigin",
    "dateTime",
    "encType",
    "formAction",
    "formEncType",
    "formMethod",
    "formNoValidate",
    "formTarget",
    "frameBorder",
    "hrefLang",
    "httpEquiv",
    "imageSizes",
    "imageSrcSet",
    "inputMode",
    "isMap",
    "itemProp",
    "itemRef",
    "itemScope",
    "itemType",
    "keyParams",
    "keyType",
    "marginHeight",
    "marginWidth",
    "maxLength",
    "mediaGroup",
    "minLength",
    "noModule",
    "noValidate",
    "playsInline",
    "radioGroup",
    "readOnly",
    "referrerPolicy",
    "rowSpan",
    "spellCheck",
    "srcDoc",
    "srcLang",
    "srcSet",
    "tabIndex",
    "useMap",
];

/// The two attributes React renames outright, because their HTML spelling collides with a
/// JavaScript reserved word or with an existing DOM property.
pub(super) const RENAMED: &[(&str, &str)] = &[("class", "className"), ("for", "htmlFor")];

/// The SVG attributes whose document spelling is hyphenated.
///
/// SVG spells a large family of its own attributes in camelCase — `viewBox`,
/// `gradientUnits`, `stdDeviation` — so hyphenating a camelCase prop by rule would corrupt
/// every one of them, and `hyphenated_to_camel` is not injective anyway: `view-box` and
/// `viewBox` both camel-case to `viewBox`. Only a list can separate the two families.
///
/// The list is not "the names seen so far". It is closed and derivable: a presentation
/// attribute is by definition an SVG attribute that is also a CSS property, so the
/// hyphenated family is exactly the multi-word presentation attributes. Anything absent
/// falls through to identity, which is what preserves `viewBox`.
pub(super) const HYPHENATED: &[&str] = &[
    "alignment-baseline",
    "baseline-shift",
    "clip-path",
    "clip-rule",
    "color-interpolation",
    "color-interpolation-filters",
    "dominant-baseline",
    "fill-opacity",
    "fill-rule",
    "flood-color",
    "flood-opacity",
    "font-family",
    "font-size",
    "font-size-adjust",
    "font-stretch",
    "font-style",
    "font-variant",
    "font-weight",
    "glyph-orientation-horizontal",
    "glyph-orientation-vertical",
    "image-rendering",
    "letter-spacing",
    "lighting-color",
    "marker-end",
    "marker-mid",
    "marker-start",
    "mask-type",
    "paint-order",
    "pointer-events",
    "shape-rendering",
    "stop-color",
    "stop-opacity",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-miterlimit",
    "stroke-opacity",
    "stroke-width",
    "text-anchor",
    "text-decoration",
    "text-overflow",
    "text-rendering",
    "transform-origin",
    "unicode-bidi",
    "vector-effect",
    "white-space",
    "word-spacing",
    "writing-mode",
];

