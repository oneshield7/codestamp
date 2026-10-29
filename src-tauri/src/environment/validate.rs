use std::collections::HashSet;

use super::models::VariableInput;
use super::resolve::is_valid_name;
use crate::error::AppError;
use crate::request::rules::MAX_KEY_VALUE_LENGTH;
use crate::storage::validate::check_id;

pub const MAX_VARIABLES: usize = 200;

pub fn check_variables(inputs: Vec<VariableInput>) -> Result<Vec<VariableInput>, AppError> {
    if inputs.len() > MAX_VARIABLES {
        return Err(AppError::invalid(
            "An environment can hold up to 200 variables",
        ));
    }
    let mut seen = HashSet::with_capacity(inputs.len());
    inputs
        .into_iter()
        .map(|input| {
            let key = input.key.trim().to_owned();
            if !is_valid_name(&key) {
                return Err(AppError::invalid(
                    "Variable names may only use letters, numbers, _ - and .",
                ));
            }
            if !seen.insert(key.clone()) {
                return Err(AppError::invalid("Each variable name must be unique"));
            }
            if let Some(id) = &input.id {
                check_id(id)?;
            }
            if let Some(group) = &input.restricted {
                check_id(group)?;
                if !input.is_secret {
                    return Err(AppError::invalid(
                        "A restricted variable keeps its value secret",
                    ));
                }
            }
            match &input.value {
                Some(value) if value.len() > MAX_KEY_VALUE_LENGTH => {
                    Err(AppError::invalid("A variable value is too long"))
                }
                None if !input.is_secret => Err(AppError::invalid("Enter a value")),
                _ => Ok(VariableInput { key, ..input }),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(key: &str, value: Option<&str>, is_secret: bool) -> VariableInput {
        VariableInput {
            id: None,
            key: key.to_owned(),
            value: value.map(str::to_owned),
            is_secret,
            enabled: true,
            restricted: None,
        }
    }

    #[test]
    fn trims_keys_and_accepts_unchanged_secrets() {
        let checked = check_variables(vec![
            input(" baseUrl ", Some("https://x"), false),
            input("token", None, true),
        ]);
        assert!(checked.is_ok_and(|vars| vars.first().is_some_and(|v| v.key == "baseUrl")));
    }

    #[test]
    fn rejects_bad_names_duplicates_and_missing_text_values() {
        assert!(check_variables(vec![input("has space", Some("x"), false)]).is_err());
        assert!(
            check_variables(vec![
                input("a", Some("1"), false),
                input("a", Some("2"), false)
            ])
            .is_err()
        );
        assert!(check_variables(vec![input("a", None, false)]).is_err());
    }
}
