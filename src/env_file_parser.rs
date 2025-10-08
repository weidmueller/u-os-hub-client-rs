// SPDX-FileCopyrightText: 2025 Weidmueller Interface GmbH & Co. KG <oss@weidmueller.com>
//
// SPDX-License-Identifier: MIT

//! Parser for environment files.
use anyhow::Result;
use std::{collections::HashMap, path::Path};
use tokio::fs;

/// Read an env file and parse it into a HashMap
pub async fn read_and_parse_env_file(path: impl AsRef<Path>) -> Result<HashMap<String, String>> {
    let file_content = fs::read_to_string(path).await?;
    Ok(parse_env_file(&file_content))
}

/// Parse an env file formatted string to a HashMap.
pub fn parse_env_file(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in input.lines() {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if let [key, value] = parts.as_slice() {
            map.insert((*key).to_string(), (*value).trim_matches('"').to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case::test_one_variable("KEY_1=valuE1!", HashMap::from([("KEY_1".to_string(), "valuE1!".to_string())]))]
    #[case::test_multiple_variables(
r#"
KEY_1=valuE1!
KEY_2=valuE2!
"#, HashMap::from([
        ("KEY_1".to_string(), "valuE1!".to_string()),
        ("KEY_2".to_string(), "valuE2!".to_string())
    ]))]
    fn test_parse_env_file(#[case] input: &str, #[case] expected_result: HashMap<String, String>) {
        let result = parse_env_file(input);

        assert_eq!(result, expected_result);
    }
}
