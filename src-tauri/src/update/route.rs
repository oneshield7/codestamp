use serde::Serialize;
use tauri::utils::config::BundleType;

/// Only used to read the version for a .deb install, which never downloads it.
pub const APT_TARGET: &str = "linux-x86_64-appimage";

/// How an available update gets installed on this copy of Codestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateRoute {
    App,
    /// A .deb belongs to apt, which installs the new version from the APT repository.
    Apt,
}

pub fn route_for(bundle: Option<BundleType>) -> UpdateRoute {
    match bundle {
        Some(BundleType::Deb) => UpdateRoute::Apt,
        _ => UpdateRoute::App,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deb_installs_update_through_apt() {
        assert_eq!(route_for(Some(BundleType::Deb)), UpdateRoute::Apt);
        for other in [
            Some(BundleType::AppImage),
            Some(BundleType::Nsis),
            Some(BundleType::Rpm),
            None,
        ] {
            assert_eq!(route_for(other), UpdateRoute::App);
        }
    }

    #[test]
    fn route_reaches_the_app_in_lowercase() {
        assert_eq!(
            serde_json::to_value(UpdateRoute::Apt).expect("route"),
            serde_json::json!("apt")
        );
        assert_eq!(
            serde_json::to_value(UpdateRoute::App).expect("route"),
            serde_json::json!("app")
        );
    }
}
