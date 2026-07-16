use crate::model::Viewport;
use anyhow::{Result, bail};

pub fn parse_viewports(input: &str) -> Result<Vec<Viewport>> {
    let mut values = Vec::new();
    for value in input.split(',').map(str::trim).filter(|v| !v.is_empty()) {
        let Some((width, height)) = value.split_once('x') else {
            bail!("invalid viewport: {value}");
        };
        let width = width.parse::<u32>()?;
        let height = height.parse::<u32>()?;
        if width < 240 || height < 240 {
            bail!("viewport is too small: {value}");
        }
        values.push(Viewport {
            width,
            height,
            dpr: 1.0,
        });
    }
    if values.is_empty() {
        bail!("at least one viewport is required");
    }
    values.sort_by_key(|viewport| std::cmp::Reverse(viewport.width));
    if values.windows(2).any(|pair| pair[0].width == pair[1].width) {
        bail!("viewport widths must be unique");
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_viewport_matrix() {
        let result = parse_viewports("390x844,1920x1080,768x1024").unwrap();
        assert_eq!(result[0].width, 1920);
        assert_eq!(result[1].width, 768);
        assert_eq!(result[2].width, 390);
    }

    #[test]
    fn rejects_duplicate_widths() {
        let result = parse_viewports("390x844,390x700");
        assert!(result.is_err());
    }

    #[test]
    fn preserves_viewport_height() {
        let result = parse_viewports("1440x900,390x844").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].height, 844);
    }

    #[test]
    fn rejects_malformed_viewports() {
        assert!(parse_viewports("wide").is_err());
        assert!(parse_viewports("100x100").is_err());
    }
}
