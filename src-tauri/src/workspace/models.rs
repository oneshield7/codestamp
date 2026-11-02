use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    /// One of the title icon colors, such as `blue`. It colors the letter and emoji icons.
    pub hue: String,
    pub icon: WorkspaceIcon,
    pub collection_count: i64,
    /// Named environments, Globals not included.
    pub environment_count: i64,
    pub flow_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceList {
    pub workspaces: Vec<Workspace>,
    pub current_id: String,
}

/// How a workspace is drawn beside its name. An image arrives and leaves as a `data:` URL, the
/// only kind of image the app's content security policy lets the window show.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WorkspaceIcon {
    Letter,
    Emoji {
        emoji: String,
    },
    /// A logo found from a website (`source`, such as `telkom.co.id`) or a file the user chose.
    Image {
        data_url: String,
        source: Option<String>,
    },
}

/// A logo the lookup found, offered for the user to pick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoCandidate {
    pub website: String,
    pub data_url: String,
}
