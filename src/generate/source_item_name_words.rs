pub fn month_date(value: &str) -> bool {
    const MONTHS: [&str; 12] = [
        "Jan ", "Feb ", "Mar ", "Apr ", "May ", "Jun ", "Jul ", "Aug ", "Sep ", "Oct ", "Nov ",
        "Dec ",
    ];
    MONTHS.iter().any(|month| value.starts_with(month))
}

pub fn pascal(value: &str) -> String {
    words(value)
        .into_iter()
        .map(|word| {
            let mut characters = word.chars();
            characters
                .next()
                .into_iter()
                .flat_map(char::to_uppercase)
                .chain(characters.flat_map(char::to_lowercase))
                .collect::<String>()
        })
        .collect()
}

pub fn camel(value: &str) -> String {
    let pascal = pascal(value);
    let mut characters = pascal.chars();
    characters
        .next()
        .into_iter()
        .flat_map(char::to_lowercase)
        .chain(characters)
        .collect()
}

fn words(value: &str) -> Vec<&str> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}
