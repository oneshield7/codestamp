use serde::{Deserialize, Serialize};

/// The bundled font of each slot (Geist for the interface, Geist Mono for code).
pub const DEFAULT_FONT: &str = "default";

/// Colour theme; `System` follows the light or dark setting of the operating system.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    #[default]
    Light,
    Dark,
    Night,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Appearance {
    /// Saved before themes existed means light, the only look back then.
    #[serde(default)]
    pub theme: Theme,
    /// `default`, a system font preset, or the id of an imported font.
    pub ui_font: String,
    pub ui_font_size: u8,
    pub code_font: String,
    pub code_font_size: u8,
    pub code_line_height: f64,
    pub ligatures: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            ui_font: DEFAULT_FONT.to_owned(),
            ui_font_size: 14,
            code_font: DEFAULT_FONT.to_owned(),
            code_font_size: 13,
            code_line_height: 1.6,
            ligatures: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedFont {
    pub id: String,
    pub name: String,
    pub format: String,
    pub size: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceState {
    pub appearance: Appearance,
    pub fonts: Vec<ImportedFont>,
}
