//! Emoji icons for collections. The catalogue is the list every chosen icon is checked against;
//! a collection without a chosen icon shows one picked from its name.

use serde::Serialize;

use crate::error::AppError;

const RELATED: usize = 6;

/// Each emoji with the words it is found by; the first word is its label.
const CATALOGUE: [(&str, &str); 58] = [
    ("💳", "card credit payment billing"),
    ("💰", "money finance bag"),
    ("🧾", "receipt invoice"),
    ("🏦", "bank finance"),
    ("💸", "transfer money payout"),
    ("🪙", "coin crypto token"),
    ("🛒", "cart shop store"),
    ("📦", "package order shipping"),
    ("🚚", "truck delivery logistics"),
    ("🏷️", "tag price product"),
    ("👤", "user account profile"),
    ("👥", "team users members"),
    ("🔐", "lock auth login"),
    ("🔑", "key token secret"),
    ("🛡️", "shield security"),
    ("✉️", "mail email"),
    ("📮", "postbox mailbox mail"),
    ("💬", "chat message"),
    ("🔔", "bell notification alert"),
    ("📣", "megaphone marketing announce"),
    ("📊", "chart analytics report"),
    ("📈", "growth metrics trend"),
    ("🗂️", "files storage folders"),
    ("🔎", "search find"),
    ("🧪", "test lab qa"),
    ("🐛", "bug debug"),
    ("🛠️", "tools internal admin"),
    ("⚙️", "settings config gear"),
    ("🧰", "toolbox kit"),
    ("🤖", "robot ai bot"),
    ("🧠", "brain model ml"),
    ("🔮", "crystal prediction"),
    ("🌐", "web globe site"),
    ("⚡", "lightning realtime fast"),
    ("🛰️", "satellite remote"),
    ("🗺️", "map geo"),
    ("📍", "pin location place"),
    ("🧭", "compass navigation"),
    ("🌦️", "weather"),
    ("✈️", "plane travel flight"),
    ("🍔", "food burger"),
    ("☕", "coffee"),
    ("🎮", "game"),
    ("🎯", "target goal"),
    ("🎵", "music audio"),
    ("🎬", "video film stream"),
    ("📚", "books course learn"),
    ("🩺", "health medical"),
    ("📅", "calendar event schedule"),
    ("🚀", "rocket launch"),
    ("🧩", "puzzle plugin integration"),
    ("🌱", "sprout seed"),
    ("🔭", "telescope explore"),
    ("🪐", "planet space"),
    ("🦊", "fox"),
    ("🐙", "octopus"),
    ("🌊", "wave ocean"),
    ("🍀", "clover luck"),
];

/// Words in a name that point to an emoji, with the ones offered next to it.
type Rule = (
    &'static [&'static str],
    &'static str,
    [&'static str; RELATED],
);
const RULES: [Rule; 26] = [
    (
        &["payment", "billing", "invoice", "checkout"],
        "💳",
        ["💳", "💰", "🧾", "🏦", "💸", "🪙"],
    ),
    (
        &["bank", "finance", "money", "wallet"],
        "🏦",
        ["🏦", "💰", "💳", "🪙", "📈", "🧾"],
    ),
    (
        &["shop", "store", "cart", "order", "commerce", "product"],
        "🛒",
        ["🛒", "📦", "🏷️", "🚚", "💳", "🧾"],
    ),
    (
        &["user", "account", "member", "profile", "customer"],
        "👤",
        ["👤", "👥", "🔐", "🔑", "✉️", "💬"],
    ),
    (
        &["auth", "login", "token", "identity", "oauth", "sso"],
        "🔐",
        ["🔐", "🔑", "🛡️", "👤", "👥", "⚙️"],
    ),
    (
        &["mail", "email", "inbox"],
        "✉️",
        ["✉️", "📮", "💬", "🔔", "📣", "👤"],
    ),
    (
        &["chat", "message", "sms"],
        "💬",
        ["💬", "✉️", "🔔", "📣", "👥", "🤖"],
    ),
    (
        &["analytics", "metric", "report", "stats", "dashboard"],
        "📊",
        ["📊", "📈", "🎯", "🔎", "🧠", "📅"],
    ),
    (
        &["internal", "admin", "tool", "ops"],
        "🛠️",
        ["🛠️", "🧰", "⚙️", "🐛", "🧪", "🛡️"],
    ),
    (
        &["test", "qa", "sandbox", "mock"],
        "🧪",
        ["🧪", "🐛", "🛠️", "🎯", "🧩", "⚙️"],
    ),
    (
        &["ai", "ml", "llm", "bot", "model"],
        "🤖",
        ["🤖", "🧠", "🔮", "⚡", "💬", "🧪"],
    ),
    (
        &["map", "geo", "location", "place"],
        "🗺️",
        ["🗺️", "🧭", "📍", "✈️", "🌐", "🛰️"],
    ),
    (&["weather"], "🌦️", ["🌦️", "🌊", "🌱", "🛰️", "🗺️", "📅"]),
    (
        &["travel", "flight", "hotel", "booking", "trip"],
        "✈️",
        ["✈️", "🗺️", "🧭", "📅", "🏷️", "💳"],
    ),
    (
        &["food", "recipe", "restaurant", "menu"],
        "🍔",
        ["🍔", "☕", "🛒", "🚚", "🧾", "📦"],
    ),
    (&["game"], "🎮", ["🎮", "🎯", "🧩", "🚀", "🪐", "🦊"]),
    (
        &["music", "audio", "podcast"],
        "🎵",
        ["🎵", "🎬", "📣", "⚡", "🌊", "🔔"],
    ),
    (
        &["video", "stream", "movie"],
        "🎬",
        ["🎬", "🎵", "⚡", "🌐", "📣", "🛰️"],
    ),
    (
        &["course", "learn", "school"],
        "📚",
        ["📚", "🧠", "🎯", "🧩", "📅", "🌱"],
    ),
    (
        &["health", "medical", "clinic"],
        "🩺",
        ["🩺", "🧪", "🛡️", "📅", "🌱", "🧠"],
    ),
    (
        &["event", "calendar", "schedule"],
        "📅",
        ["📅", "🔔", "🎯", "📣", "✉️", "👥"],
    ),
    (
        &["shipping", "delivery", "logistics"],
        "🚚",
        ["🚚", "📦", "🗺️", "🧭", "🛒", "🏷️"],
    ),
    (
        &["web", "site", "cms", "blog"],
        "🌐",
        ["🌐", "📣", "🔎", "🧩", "⚡", "🗂️"],
    ),
    (
        &["realtime", "socket", "live"],
        "⚡",
        ["⚡", "🛰️", "🌐", "🔔", "💬", "🚀"],
    ),
    (
        &["file", "upload", "storage", "media", "asset"],
        "🗂️",
        ["🗂️", "📦", "🔎", "🛡️", "🌐", "🧰"],
    ),
    (&["search"], "🔎", ["🔎", "🧭", "🗂️", "🧠", "🎯", "🌐"]),
];

/// For names no rule recognises; the name's hash picks one, so it never changes on its own.
const POOL: [&str; 16] = [
    "📮", "🚀", "🧭", "🧩", "🌱", "🔭", "🪐", "🦊", "🐙", "🌊", "🎯", "🧰", "📦", "🛰️", "🍀", "🔮",
];
/// Shorter keys would match too many unrelated words as a prefix ("ai" in "airline").
const MIN_PREFIX_KEY: usize = 4;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconEntry {
    pub emoji: &'static str,
    pub label: &'static str,
    pub keywords: &'static str,
}

/// The icon a name gets automatically, and the ones the picker offers first for it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconSuggestion {
    pub emoji: &'static str,
    pub related: [&'static str; RELATED],
}

pub fn catalogue() -> Vec<IconEntry> {
    CATALOGUE
        .iter()
        .map(|(emoji, keywords)| IconEntry {
            emoji,
            label: keywords.split(' ').next().unwrap_or(keywords),
            keywords,
        })
        .collect()
}

pub fn suggest(name: &str) -> IconSuggestion {
    let lower = name.trim().to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let matches = |key: &&str| {
        words
            .iter()
            .any(|word| word == key || (key.len() >= MIN_PREFIX_KEY && word.starts_with(key)))
    };
    if let Some((_, emoji, related)) = RULES.iter().find(|(keys, _, _)| keys.iter().any(matches)) {
        return IconSuggestion {
            emoji,
            related: *related,
        };
    }
    let hash = lower.chars().fold(0_u32, |hash, c| {
        hash.wrapping_mul(31).wrapping_add(u32::from(c))
    });
    let start = usize::try_from(hash).unwrap_or(0) % POOL.len();
    IconSuggestion {
        emoji: POOL[start],
        related: std::array::from_fn(|step| POOL[(start + step) % POOL.len()]),
    }
}

/// The chosen icon when it is still in the catalogue, else the name's automatic one.
pub fn resolve(name: &str, chosen: Option<&str>) -> String {
    chosen
        .and_then(|icon| check_icon(icon).ok())
        .unwrap_or_else(|| suggest(name).emoji)
        .to_owned()
}

pub fn check_icon(raw: &str) -> Result<&'static str, AppError> {
    CATALOGUE
        .iter()
        .find(|(emoji, _)| *emoji == raw)
        .map(|(emoji, _)| *emoji)
        .ok_or_else(|| AppError::invalid("Choose an icon from the list"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_offered_icon_is_in_the_catalogue() {
        let offered = RULES
            .iter()
            .flat_map(|(_, emoji, related)| std::iter::once(*emoji).chain(related.iter().copied()))
            .chain(POOL);
        for emoji in offered {
            assert!(
                check_icon(emoji).is_ok(),
                "{emoji} is offered but not in the catalogue"
            );
        }
    }

    #[test]
    fn names_get_a_matching_or_stable_icon() {
        assert_eq!(suggest("Payments API").emoji, "💳");
        assert_eq!(suggest("Shop API").emoji, "🛒");
        assert_eq!(suggest("Internal tools").emoji, "🛠️");
        assert_eq!(suggest("  USER accounts ").emoji, "👤");
        // "airline" must not match the short key "ai".
        assert_ne!(suggest("Airline").emoji, "🤖");
        assert_eq!(suggest("Kestrel").emoji, "🍀");
        assert_eq!(suggest("My API").emoji, "🪐");
        assert_eq!(suggest("Kestrel").emoji, suggest("kestrel").emoji);
    }

    #[test]
    fn only_catalogue_icons_are_accepted() {
        assert_eq!(check_icon("💳").ok(), Some("💳"));
        assert!(check_icon("<script>").is_err());
        assert!(check_icon("💳💳").is_err());
        assert!(check_icon("").is_err());
        assert_eq!(resolve("Shop API", Some("not an icon")), "🛒");
        assert_eq!(resolve("Shop API", Some("🦊")), "🦊");
    }
}
