//! The name tables `jsx_attr_names` reads in both directions.
//!
//! They live beside the rule rather than inside it only to keep each file readable. There
//! is still exactly one copy of each list, which is what stops the JSX spelling and the
//! document spelling of a name from drifting apart.
/// The XML namespace prefixes that reach an HTML document, and the only ones whose JSX
/// spelling can be split back apart.
///
/// The forward direction needs no list at all: a colon is an explicit word boundary, so
/// camel-joining on it answers for every prefix, including the ones Alpine and htmx invent.
/// The inverse cannot be a rule, because the boundary is gone — splitting `xlinkHref` at
/// its case change is the same operation that would turn `dateTime` into `date:time`.
///
/// A prefix matches only when an uppercase letter follows it, which is what makes the order
/// of this list irrelevant: `xmlnsXlink` also begins with `xml`, and a plain prefix search
/// would split it as `xml:nsXlink` depending on nothing but which entry came first.
pub(super) const NAMESPACE_PREFIXES: &[&str] = &["xlink", "xml", "xmlns"];

/// React's canonical prop names for HTML attributes that differ from their HTML spelling
/// only by case. Taken from the React DOM attribute reference.
///
/// Only by case: an entry whose document spelling also carries a hyphen does not belong
/// here, because `document_attribute` restores an entry by lowercasing it and would emit a
/// name that never existed. `accept-charset` and `http-equiv` are React's only two such
/// names, and they live in `HYPHENATED` with the rest of the hyphenated family.
pub(super) const CAMEL_CASED: &[&str] = &[
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

/// The attributes whose document spelling is hyphenated and whose JSX spelling is not.
///
/// SVG spells a large family of its own attributes in camelCase — `viewBox`,
/// `gradientUnits`, `stdDeviation` — so hyphenating a camelCase prop by rule would corrupt
/// every one of them, and `hyphenated_to_camel` is not injective anyway: `view-box` and
/// `viewBox` both camel-case to `viewBox`. Only a list can separate the two families.
///
/// The list does not have to be complete, and cannot be. A presentation attribute is
/// defined by reference to the CSS property set, so the family grows with each revision of
/// the styling module, and a list transcribed from a current reference simultaneously
/// misses everything the previous revision defined and browsers still parse —
/// `color-rendering` and `enable-background` among them. Both directions read this list, so
/// a name it does not hold keeps its captured spelling on the way out and on the way back,
/// and an omission costs nothing rather than costing the name.
pub(super) const HYPHENATED: &[&str] = &[
    // React's only two hyphenated HTML attributes. They belong to the same family as the
    // SVG names below: hyphenated in the document, camelCase as a prop.
    "accept-charset",
    "http-equiv",
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
