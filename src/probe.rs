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
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_viewport_matrix() {
        let result = parse_viewports("1440x900,390x844").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].width, 390);
    }

    #[test]
    fn rejects_malformed_viewports() {
        assert!(parse_viewports("wide").is_err());
        assert!(parse_viewports("100x100").is_err());
    }
}
