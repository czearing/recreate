pub fn equivalent(left: Option<&str>, right: Option<&str>) -> bool {
    left == right
        || matches!((left, right), (Some(left), Some(right)) if normalize_zeros(left) == normalize_zeros(right))
        || matches!((left, right), (Some(left), Some(right)) if close_numbers(left, right))
}

pub fn layout_property(property: &str) -> bool {
    property.starts_with("align-")
        || property.starts_with("flex")
        || property.starts_with("grid-")
        || property.starts_with("justify-")
        || property.starts_with("margin")
        || matches!(
            property,
            "block-size"
                | "bottom"
                | "float"
                | "height"
                | "inline-size"
                | "inset"
                | "left"
                | "max-block-size"
                | "max-height"
                | "max-inline-size"
                | "max-width"
                | "min-block-size"
                | "min-height"
                | "min-inline-size"
                | "min-width"
                | "order"
                | "perspective-origin"
                | "position"
                | "right"
                | "top"
                | "transform"
                | "transform-origin"
                | "translate"
                | "width"
        )
}

pub fn animation_property(property: &str) -> bool {
    property == "animation" || property.starts_with("animation-")
}

fn normalize_zeros(value: &str) -> String {
    let characters: Vec<_> = value.chars().collect();
    let mut normalized = String::with_capacity(value.len());
    let mut index = 0;
    while index < characters.len() {
        if number_start(&characters, index) {
            let start = index;
            index += 1;
            while index < characters.len()
                && (characters[index].is_ascii_digit() || characters[index] == '.')
            {
                index += 1;
            }
            let number: String = characters[start..index].iter().collect();
            let unit_start = index;
            while index < characters.len()
                && (characters[index].is_ascii_alphabetic() || characters[index] == '%')
            {
                index += 1;
            }
            if unit_start < index && number.parse::<f64>().is_ok_and(|value| value == 0.0) {
                normalized.push('0');
            } else {
                normalized.extend(characters[start..index].iter());
            }
            continue;
        }
        normalized.push(characters[index]);
        index += 1;
    }
    normalized
}

fn number_start(characters: &[char], index: usize) -> bool {
    let current = characters[index];
    let numeric = current.is_ascii_digit()
        || current == '.'
        || ((current == '-' || current == '+')
            && characters
                .get(index + 1)
                .is_some_and(|next| next.is_ascii_digit() || *next == '.'));
    numeric
        && (index == 0
            || characters[index - 1].is_ascii_whitespace()
            || matches!(characters[index - 1], '(' | ',' | '/'))
}

fn close_numbers(left: &str, right: &str) -> bool {
    let (left_shape, left_numbers) = number_shape(left);
    let (right_shape, right_numbers) = number_shape(right);
    left_shape == right_shape
        && left_numbers.len() == right_numbers.len()
        && left_numbers
            .iter()
            .zip(right_numbers)
            .all(|((left, left_unit), (right, right_unit))| {
                left_unit == &right_unit && (left - right).abs() <= 0.02
            })
}

fn number_shape(value: &str) -> (String, Vec<(f64, String)>) {
    let characters: Vec<_> = value.chars().collect();
    let mut shape = String::with_capacity(value.len());
    let mut numbers = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if !number_start(&characters, index) {
            shape.push(characters[index]);
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < characters.len()
            && (characters[index].is_ascii_digit() || characters[index] == '.')
        {
            index += 1;
        }
        let number: String = characters[start..index].iter().collect();
        let unit_start = index;
        while index < characters.len()
            && (characters[index].is_ascii_alphabetic() || characters[index] == '%')
        {
            index += 1;
        }
        let unit: String = characters[unit_start..index].iter().collect();
        let Ok(number) = number.parse() else {
            shape.extend(characters[start..index].iter());
            continue;
        };
        shape.push_str("{}");
        shape.push_str(&unit);
        numbers.push((number, unit));
    }
    (shape, numbers)
}

#[cfg(test)]
mod tests {
    #[test]
    fn treats_zero_units_as_equivalent() {
        assert!(super::equivalent(Some("0% 0%"), Some("0px 0px")));
        assert!(super::equivalent(
            Some("translate(0px, -0em)"),
            Some("translate(0%, 0px)")
        ));
    }

    #[test]
    fn retains_nonzero_units_and_identifiers() {
        assert!(!super::equivalent(Some("10% 0%"), Some("10px 0px")));
        assert!(!super::equivalent(Some("#000"), Some("#0")));
    }

    #[test]
    fn tolerates_browser_subpixel_serialization() {
        assert!(super::equivalent(Some("313.328px"), Some("313.336px")));
        assert!(!super::equivalent(Some("313.3px"), Some("313.4px")));
    }
}
