//! The single owner of "what is this HTML attribute called in JSX?".
//!
//! Hyphenated attributes carry their own word boundaries, so `stroke-width` becomes
//! `strokeWidth` by a rule. A large family of HTML attributes does not: they are written as
//! one lowercase word while React's canonical prop is camelCase, and nothing in `datetime`
//! says the split falls after `date` rather than after `dat`. Those names are fixed points
//! of any hyphen-driven conversion, so the conversion cannot ever produce the right answer
//! for them and only a table can.
//!
//! Three of the family used to be hand-listed beside the conversion and the rest were not,
//! which made the omission invisible until a page happened to use one — a `<time>` element
//! emitted `datetime={...}`, a prop React does not recognise. The table below is therefore
//! the whole family rather than the cases seen so far.
//!
//! It is written as one list of canonical React names, not as pairs. The HTML spelling of
//! every entry is its own lowercase form, so the two halves cannot drift apart: adding a
//! name adds both directions at once.

/// React's canonical prop names for HTML attributes that differ from their HTML spelling
/// only by case. Taken from the React DOM attribute reference.
const CAMEL_CASED: &[&str] = &[
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
const RENAMED: &[(&str, &str)] = &[("class", "className"), ("for", "htmlFor")];

/// Translates a captured HTML attribute name into the prop name React recognises.
///
/// `aria-` and `data-` attributes are passed through verbatim: React accepts those exactly
/// as authored, and camel-casing them would emit a prop no renderer knows.
pub fn jsx_attribute(name: &str) -> String {
    if name.starts_with("aria-") || name.starts_with("data-") {
        return name.into();
    }
    if let Some((_, renamed)) = RENAMED.iter().find(|(html, _)| *html == name) {
        return (*renamed).into();
    }
    if let Some(camel) = CAMEL_CASED
        .iter()
        .find(|candidate| candidate.eq_ignore_ascii_case(name))
    {
        return (*camel).into();
    }
    hyphenated_to_camel(name)
}

fn hyphenated_to_camel(value: &str) -> String {
    let mut result = String::new();
    let mut uppercase = false;
    for character in value.chars() {
        if character == '-' {
            uppercase = true;
        } else if uppercase {
            result.extend(character.to_uppercase());
            uppercase = false;
        } else {
            result.push(character);
        }
    }
    result
}

#[cfg(test)]
#[path = "jsx_attr_names_tests.rs"]
mod tests;
