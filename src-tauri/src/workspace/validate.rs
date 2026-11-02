use crate::error::AppError;

/// The title icon colors of DESIGN.md; the UI draws a workspace with `data-hue`.
const HUES: [&str; 8] = [
    "blue", "green", "teal", "violet", "amber", "orange", "rose", "slate",
];

pub fn check_hue(hue: &str) -> Result<(), AppError> {
    if HUES.contains(&hue) {
        Ok(())
    } else {
        Err(AppError::invalid("Choose one of the workspace colors"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_design_colors_are_accepted() {
        assert!(check_hue("violet").is_ok());
        assert!(check_hue("red").is_err());
        assert!(check_hue("blue; background: url(x)").is_err());
    }
}
