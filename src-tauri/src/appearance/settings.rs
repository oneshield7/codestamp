use rusqlite::Connection;
use tauri::WebviewWindow;
use tauri::window::Color;

use super::models::{Appearance, Theme};
use super::repo;
use super::validate::{BASE_UI_FONT_SIZE, check_appearance};
use crate::error::AppError;

/// Where the app keeps the theme chosen in this session (`appearance.svelte.ts`).
const THEME_KEY: &str = "codestamp.theme";

/// The saved appearance; the defaults when nothing was saved or it no longer validates
/// (for example after its imported font disappeared).
pub fn current(conn: &Connection) -> Result<Appearance, AppError> {
    let stored = repo::load_appearance(conn)?.unwrap_or_default();
    match check_appearance(conn, stored) {
        Ok(appearance) => Ok(appearance),
        Err(AppError::InvalidInput(_)) => Ok(Appearance::default()),
        Err(other) => Err(other),
    }
}

/// The interface size scales the whole window, so spacing grows together with the text.
pub fn zoom_factor(appearance: &Appearance) -> f64 {
    f64::from(appearance.ui_font_size) / f64::from(BASE_UI_FONT_SIZE)
}

/// The theme of the native title bar; `None` lets the operating system decide.
pub fn window_theme(appearance: &Appearance) -> Option<tauri::Theme> {
    match appearance.theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark | Theme::Night => Some(tauri::Theme::Dark),
    }
}

pub fn restore_window(window: &WebviewWindow, appearance: &Appearance) -> Result<(), AppError> {
    window
        .set_zoom(zoom_factor(appearance))
        .map_err(|_| AppError::Window)?;
    window
        .set_theme(window_theme(appearance))
        .map_err(|_| AppError::Window)?;
    paint_background(window, appearance)
}

/// Gives the window and its webview the page colour of the theme (`--bg` in app.css), so nothing
/// white shows before the page is drawn or while the window is resized.
pub fn paint_background(window: &WebviewWindow, appearance: &Appearance) -> Result<(), AppError> {
    let system = window.theme().map_err(|_| AppError::Window)?;
    window
        .set_background_color(Some(background(appearance.theme, system)))
        .map_err(|_| AppError::Window)
}

fn background(theme: Theme, system: tauri::Theme) -> Color {
    match (theme, system) {
        (Theme::Light, _) | (Theme::System, tauri::Theme::Light) => Color(0xf3, 0xf4, 0xf6, 0xff),
        (Theme::Night, _) => Color(0x07, 0x0a, 0x12, 0xff),
        _ => Color(0x15, 0x17, 0x1c, 0xff),
    }
}

/// Puts the theme on the page as soon as it exists, so the launch screen in `index.html` is drawn
/// in it before any of the app's own code has run. A reload uses the theme chosen since launch,
/// which the app keeps in session storage. `System` is resolved the way the app does.
pub fn theme_script(appearance: &Appearance) -> Result<String, AppError> {
    let theme = serde_json::to_string(&appearance.theme).map_err(|_| AppError::Storage)?;
    Ok(format!(
        "(() => {{
  let saved = {theme};
  try {{
    const chosen = sessionStorage.getItem('{THEME_KEY}');
    if (['system', 'light', 'dark', 'night'].includes(chosen)) saved = chosen;
  }} catch {{
    saved = {theme};
  }}
  const theme = saved !== 'system' ? saved
    : matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  const apply = () => {{ document.documentElement.dataset.theme = theme; }};
  if (document.documentElement) {{
    apply();
    return;
  }}
  new MutationObserver((_, observer) => {{
    if (document.documentElement) {{
      apply();
      observer.disconnect();
    }}
  }}).observe(document, {{ childList: true }});
}})();"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;

    #[test]
    fn falls_back_to_defaults_when_the_saved_font_is_gone() {
        let db = Database::in_memory().expect("database");
        let missing_font = Appearance {
            ui_font: "8f2c1a7e-0000-4000-8000-000000000003".to_owned(),
            ui_font_size: 16,
            ..Appearance::default()
        };
        let loaded = db.with(|conn| {
            repo::save_appearance(conn, &missing_font)?;
            current(conn)
        });
        assert_eq!(loaded.ok(), Some(Appearance::default()));
    }

    #[test]
    fn reads_appearance_saved_before_themes_as_light() {
        let saved = r#"{"uiFont":"default","uiFontSize":14,"codeFont":"default","codeFontSize":13,"codeLineHeight":1.6,"ligatures":true}"#;
        let parsed: Result<Appearance, _> = serde_json::from_str(saved);
        assert_eq!(parsed.ok().map(|a| a.theme), Some(Theme::Light));
        let unknown: Result<Appearance, _> = serde_json::from_str(
            &saved.replace("\"ligatures\"", "\"theme\":\"sepia\",\"ligatures\""),
        );
        assert!(unknown.is_err());
    }

    #[test]
    fn the_launch_script_names_the_saved_theme() {
        let night = Appearance {
            theme: Theme::Night,
            ..Appearance::default()
        };
        let script = theme_script(&night).expect("script");
        assert!(script.contains("let saved = \"night\";"));
        assert_eq!(
            background(Theme::System, tauri::Theme::Dark),
            background(Theme::Dark, tauri::Theme::Light)
        );
        assert_ne!(
            background(Theme::System, tauri::Theme::Light),
            background(Theme::Night, tauri::Theme::Light)
        );
    }

    #[test]
    fn dark_themes_get_a_dark_title_bar() {
        let night = Appearance {
            theme: Theme::Night,
            ..Appearance::default()
        };
        assert_eq!(window_theme(&night), Some(tauri::Theme::Dark));
        let system = Appearance {
            theme: Theme::System,
            ..Appearance::default()
        };
        assert_eq!(window_theme(&system), None);
    }

    #[test]
    fn zooms_relative_to_the_base_size() {
        let bigger = Appearance {
            ui_font_size: 21,
            ..Appearance::default()
        };
        assert!((zoom_factor(&bigger) - 1.5).abs() < f64::EPSILON);
        assert!((zoom_factor(&Appearance::default()) - 1.0).abs() < f64::EPSILON);
    }
}
