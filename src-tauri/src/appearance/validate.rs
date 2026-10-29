use std::ops::RangeInclusive;

use rusqlite::Connection;

use super::models::{Appearance, DEFAULT_FONT};
use super::repo;
use crate::error::AppError;
use crate::storage::validate::check_id;

/// The interface font size at which the window is shown at 100 %.
pub const BASE_UI_FONT_SIZE: u8 = 14;
const UI_PRESETS: [&str; 2] = [DEFAULT_FONT, "segoe-ui"];
const CODE_PRESETS: [&str; 3] = [DEFAULT_FONT, "cascadia-code", "consolas"];
const UI_FONT_SIZES: RangeInclusive<u8> = 11..=20;
const CODE_FONT_SIZES: RangeInclusive<u8> = 10..=24;
const LINE_HEIGHTS: RangeInclusive<f64> = 1.2..=2.2;

pub fn check_appearance(conn: &Connection, input: Appearance) -> Result<Appearance, AppError> {
    check_font_choice(conn, &input.ui_font, &UI_PRESETS)?;
    check_font_choice(conn, &input.code_font, &CODE_PRESETS)?;
    if !UI_FONT_SIZES.contains(&input.ui_font_size) {
        return Err(AppError::invalid(
            "The interface font size must be 11 to 20 px",
        ));
    }
    if !CODE_FONT_SIZES.contains(&input.code_font_size) {
        return Err(AppError::invalid(
            "The editor font size must be 10 to 24 px",
        ));
    }
    if !LINE_HEIGHTS.contains(&input.code_line_height) {
        return Err(AppError::invalid("The line height must be 1.2 to 2.2"));
    }
    Ok(Appearance {
        code_line_height: (input.code_line_height * 10.0).round() / 10.0,
        ..input
    })
}

/// A preset of this slot, or a font the user imported.
fn check_font_choice(conn: &Connection, choice: &str, presets: &[&str]) -> Result<(), AppError> {
    if presets.contains(&choice) {
        return Ok(());
    }
    check_id(choice).map_err(|_| AppError::invalid("Unknown font"))?;
    if repo::font_format(conn, choice)?.is_none() {
        return Err(AppError::invalid("That font is no longer available"));
    }
    Ok(())
}

/// Detects the font format from the file's first bytes; the file extension is not trusted.
pub fn font_format(bytes: &[u8]) -> Option<&'static str> {
    match bytes.get(..4)? {
        [0x00, 0x01, 0x00, 0x00] | b"true" => Some("ttf"),
        b"OTTO" => Some("otf"),
        b"wOF2" => Some("woff2"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::models::ImportedFont;
    use crate::storage::db::Database;

    const FONT_ID: &str = "8f2c1a7e-0000-4000-8000-000000000002";

    fn check(input: Appearance) -> Result<Appearance, AppError> {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            repo::insert_font(
                conn,
                &ImportedFont {
                    id: FONT_ID.to_owned(),
                    name: "Fira Code".to_owned(),
                    format: "ttf".to_owned(),
                    size: 10,
                },
            )?;
            check_appearance(conn, input)
        })
    }

    #[test]
    fn accepts_defaults_presets_and_imported_fonts() {
        assert!(check(Appearance::default()).is_ok());
        assert!(
            check(Appearance {
                ui_font: "segoe-ui".to_owned(),
                code_font: FONT_ID.to_owned(),
                ..Appearance::default()
            })
            .is_ok()
        );
    }

    #[test]
    fn rejects_unknown_fonts() {
        for font in [
            "Comic Sans",
            "consolas",
            "8f2c1a7e-0000-4000-8000-0000000000ff",
        ] {
            let input = Appearance {
                ui_font: font.to_owned(),
                ..Appearance::default()
            };
            assert!(check(input).is_err(), "{font}");
        }
    }

    #[test]
    fn rejects_sizes_and_line_heights_out_of_range() {
        let too_small = Appearance {
            ui_font_size: 10,
            ..Appearance::default()
        };
        let too_big = Appearance {
            code_font_size: 25,
            ..Appearance::default()
        };
        let not_a_number = Appearance {
            code_line_height: f64::NAN,
            ..Appearance::default()
        };
        assert!(check(too_small).is_err());
        assert!(check(too_big).is_err());
        assert!(check(not_a_number).is_err());
    }

    #[test]
    fn rounds_the_line_height_to_one_decimal() {
        let input = Appearance {
            code_line_height: 1.749_999,
            ..Appearance::default()
        };
        let rounded = check(input).map(|appearance| appearance.code_line_height);
        assert!(rounded.is_ok_and(|value| (value - 1.7).abs() < f64::EPSILON));
    }

    #[test]
    fn detects_font_formats_from_their_signature() {
        assert_eq!(font_format(&[0, 1, 0, 0, 9]), Some("ttf"));
        assert_eq!(font_format(b"OTTO...."), Some("otf"));
        assert_eq!(font_format(b"wOF2...."), Some("woff2"));
        assert_eq!(font_format(b"wOFF...."), None);
        assert_eq!(font_format(b"<svg"), None);
        assert_eq!(font_format(b"PK"), None);
    }
}
