use std::collections::HashMap;
use crate::models::{ExtractionRule, OutputTrigger};

/// Run extraction rules against stdout, returning extracted key-value pairs.
pub fn run_extractions(stdout: &str, rules: &[ExtractionRule]) -> HashMap<String, String> {
    let mut results = HashMap::new();
    for rule in rules {
        if let Some(value) = extract_value(stdout, &rule.pattern, &rule.rule_type) {
            results.insert(rule.name.clone(), value);
        }
    }
    results
}

fn extract_value(stdout: &str, pattern: &str, rule_type: &str) -> Option<String> {
    match rule_type {
        "regex" => extract_regex(stdout, pattern),
        "jsonpath" => extract_jsonpath(stdout, pattern),
        _ => None,
    }
}

fn extract_regex(stdout: &str, pattern: &str) -> Option<String> {
    let re = regex::Regex::new(pattern).ok()?;
    let caps = re.captures(stdout)?;
    // Try named groups first, then group 1
    for name in re.capture_names().flatten() {
        if let Some(m) = caps.name(name) {
            return Some(m.as_str().to_string());
        }
    }
    caps.get(1).map(|m: regex::Match| m.as_str().to_string())
}

fn extract_jsonpath(stdout: &str, path: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    // Simple dot-notation traversal: $.results.count -> ["results", "count"]
    let path = path.strip_prefix("$.").unwrap_or(path);
    let keys: Vec<&str> = path.split('.').collect();
    let mut current = &parsed;
    for key in keys {
        current = current.get(key)?;
    }
    match current {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}

/// Run trigger patterns against stdout and stderr, returning matched (pattern, severity) pairs.
pub fn run_triggers(stdout: &str, stderr: &str, triggers: &[OutputTrigger]) -> Vec<(String, String)> {
    let mut matches = Vec::new();
    for trigger in triggers {
        let matched = if let Ok(re) = regex::Regex::new(&trigger.pattern) {
            re.is_match(stdout) || re.is_match(stderr)
        } else {
            // Fall back to substring match
            stdout.contains(&trigger.pattern) || stderr.contains(&trigger.pattern)
        };
        if matched {
            matches.push((trigger.pattern.clone(), trigger.severity.clone()));
        }
    }
    matches
}
