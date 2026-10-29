use uuid::Uuid;

use crate::storage::db::now_ms;

/// Built-in `{{$name}}` values, made fresh for every reference (the same names as Postman's).
/// The descriptions double as autocomplete hints.
pub const DYNAMIC_VARIABLES: [(&str, &str); 7] = [
    ("$guid", "A random UUID v4"),
    ("$randomUUID", "A random UUID v4"),
    ("$timestamp", "Current Unix time in seconds"),
    ("$isoTimestamp", "Current UTC time in ISO 8601"),
    ("$randomInt", "A random whole number from 0 to 1000"),
    ("$randomBoolean", "true or false"),
    ("$randomAlphaNumeric", "One random letter or digit"),
];

const ALPHANUMERIC: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

pub fn is_dynamic(name: &str) -> bool {
    DYNAMIC_VARIABLES.iter().any(|(known, _)| *known == name)
}

/// A fresh value, or `None` for a name that is not a dynamic variable.
pub fn generate(name: &str) -> Option<String> {
    let value = match name {
        "$guid" | "$randomUUID" => Uuid::new_v4().to_string(),
        "$timestamp" => now_ms().div_euclid(1000).to_string(),
        "$isoTimestamp" => iso_timestamp(now_ms()),
        "$randomInt" => (random_u32() % 1001).to_string(),
        "$randomBoolean" => (random_u32() % 2 == 1).to_string(),
        "$randomAlphaNumeric" => {
            let index = usize::try_from(random_u32()).unwrap_or_default() % ALPHANUMERIC.len();
            ALPHANUMERIC
                .get(index)
                .map(|byte| char::from(*byte).to_string())
                .unwrap_or_default()
        }
        _ => return None,
    };
    Some(value)
}

/// Uses the operating system's secure random source through a v4 UUID, whose first four
/// bytes are fully random.
fn random_u32() -> u32 {
    let bytes = Uuid::new_v4().into_bytes();
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// `2026-10-22T08:15:30.123Z`, the format Postman uses for `$isoTimestamp`.
pub fn iso_timestamp(millis: i64) -> String {
    let seconds = millis.div_euclid(1000);
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    let in_day = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        in_day / 3600,
        in_day % 3600 / 60,
        in_day % 60,
        millis.rem_euclid(1000)
    )
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_iso_timestamps() {
        assert_eq!(iso_timestamp(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_timestamp(1_700_000_000_123), "2023-11-14T22:13:20.123Z");
        assert_eq!(iso_timestamp(951_782_400_000), "2000-02-29T00:00:00.000Z");
    }

    #[test]
    fn generates_values_in_the_expected_shape() {
        let guid = generate("$guid").expect("guid");
        assert!(Uuid::parse_str(&guid).is_ok());
        assert_ne!(generate("$guid"), generate("$guid"));
        let number: u32 = generate("$randomInt")
            .expect("int")
            .parse()
            .expect("number");
        assert!(number <= 1000);
        assert!(matches!(
            generate("$randomBoolean").as_deref(),
            Some("true" | "false")
        ));
        assert_eq!(generate("$randomAlphaNumeric").map(|v| v.len()), Some(1));
        let seconds: i64 = generate("$timestamp")
            .expect("timestamp")
            .parse()
            .expect("number");
        assert!(seconds > 1_700_000_000);
        assert!(generate("$randomFirstName").is_none());
        assert!(generate("guid").is_none());
    }
}
