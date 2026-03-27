use kronforce::models::*;
use kronforce::output_rules::*;

// --- Extractions ---

#[test]
fn test_regex_extraction_group1() {
    let rules = vec![ExtractionRule {
        name: "duration".to_string(),
        pattern: r"took (\d+)ms".to_string(),
        rule_type: "regex".to_string(),
        write_to_variable: None,
    }];
    let result = run_extractions("Processing took 245ms total", &rules);
    assert_eq!(result.get("duration").unwrap(), "245");
}

#[test]
fn test_regex_extraction_no_match() {
    let rules = vec![ExtractionRule {
        name: "missing".to_string(),
        pattern: r"not_found_(\d+)".to_string(),
        rule_type: "regex".to_string(),
        write_to_variable: None,
    }];
    let result = run_extractions("no match here", &rules);
    assert!(!result.contains_key("missing"));
}

#[test]
fn test_regex_extraction_multiple_rules() {
    let rules = vec![
        ExtractionRule {
            name: "status".to_string(),
            pattern: r"status: (\w+)".to_string(),
            rule_type: "regex".to_string(),
            write_to_variable: None,
        },
        ExtractionRule {
            name: "count".to_string(),
            pattern: r"processed (\d+) records".to_string(),
            rule_type: "regex".to_string(),
            write_to_variable: None,
        },
    ];
    let result = run_extractions("status: healthy, processed 42 records", &rules);
    assert_eq!(result.get("status").unwrap(), "healthy");
    assert_eq!(result.get("count").unwrap(), "42");
}

#[test]
fn test_jsonpath_extraction() {
    let rules = vec![ExtractionRule {
        name: "count".to_string(),
        pattern: "$.results.count".to_string(),
        rule_type: "jsonpath".to_string(),
        write_to_variable: None,
    }];
    let result = run_extractions(r#"{"results": {"count": 42}}"#, &rules);
    assert_eq!(result.get("count").unwrap(), "42");
}

#[test]
fn test_jsonpath_extraction_string_value() {
    let rules = vec![ExtractionRule {
        name: "status".to_string(),
        pattern: "$.status".to_string(),
        rule_type: "jsonpath".to_string(),
        write_to_variable: None,
    }];
    let result = run_extractions(r#"{"status": "healthy"}"#, &rules);
    assert_eq!(result.get("status").unwrap(), "healthy");
}

#[test]
fn test_jsonpath_extraction_invalid_json() {
    let rules = vec![ExtractionRule {
        name: "val".to_string(),
        pattern: "$.key".to_string(),
        rule_type: "jsonpath".to_string(),
        write_to_variable: None,
    }];
    let result = run_extractions("not json at all", &rules);
    assert!(!result.contains_key("val"));
}

#[test]
fn test_jsonpath_extraction_missing_path() {
    let rules = vec![ExtractionRule {
        name: "val".to_string(),
        pattern: "$.nonexistent.path".to_string(),
        rule_type: "jsonpath".to_string(),
        write_to_variable: None,
    }];
    let result = run_extractions(r#"{"other": "data"}"#, &rules);
    assert!(!result.contains_key("val"));
}

// --- Triggers ---

#[test]
fn test_trigger_matches_stdout() {
    let triggers = vec![OutputTrigger {
        pattern: "ERROR".to_string(),
        severity: "error".to_string(),
    }];
    let matches = run_triggers("something ERROR happened", "", &triggers);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].0, "ERROR");
    assert_eq!(matches[0].1, "error");
}

#[test]
fn test_trigger_matches_stderr() {
    let triggers = vec![OutputTrigger {
        pattern: "WARN".to_string(),
        severity: "warning".to_string(),
    }];
    let matches = run_triggers("", "WARN: low memory", &triggers);
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_trigger_no_match() {
    let triggers = vec![OutputTrigger {
        pattern: "CRITICAL".to_string(),
        severity: "error".to_string(),
    }];
    let matches = run_triggers("everything is fine", "", &triggers);
    assert_eq!(matches.len(), 0);
}

#[test]
fn test_trigger_regex_pattern() {
    let triggers = vec![OutputTrigger {
        pattern: r"ERROR|FATAL".to_string(),
        severity: "error".to_string(),
    }];
    let matches = run_triggers("FATAL: disk full", "", &triggers);
    assert_eq!(matches.len(), 1);
}

#[test]
fn test_trigger_multiple_matches() {
    let triggers = vec![
        OutputTrigger {
            pattern: "ERROR".to_string(),
            severity: "error".to_string(),
        },
        OutputTrigger {
            pattern: "WARNING".to_string(),
            severity: "warning".to_string(),
        },
    ];
    let matches = run_triggers("ERROR and WARNING found", "", &triggers);
    assert_eq!(matches.len(), 2);
}

// --- Assertions ---

#[test]
fn test_assertion_pattern_found() {
    let assertions = vec![OutputAssertion {
        pattern: "OK".to_string(),
        message: None,
    }];
    let failures = run_assertions("Status: OK", &assertions);
    assert_eq!(failures.len(), 0);
}

#[test]
fn test_assertion_pattern_not_found() {
    let assertions = vec![OutputAssertion {
        pattern: "OK".to_string(),
        message: Some("expected OK".to_string()),
    }];
    let failures = run_assertions("Status: FAIL", &assertions);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0], "expected OK");
}

#[test]
fn test_assertion_default_message() {
    let assertions = vec![OutputAssertion {
        pattern: "healthy".to_string(),
        message: None,
    }];
    let failures = run_assertions("Status: down", &assertions);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("healthy"));
    assert!(failures[0].contains("not found"));
}

#[test]
fn test_assertion_regex_pattern() {
    let assertions = vec![OutputAssertion {
        pattern: r"records: \d+".to_string(),
        message: None,
    }];
    let pass = run_assertions("processed records: 42", &assertions);
    assert_eq!(pass.len(), 0);

    let fail = run_assertions("no records info", &assertions);
    assert_eq!(fail.len(), 1);
}

#[test]
fn test_multiple_assertions() {
    let assertions = vec![
        OutputAssertion {
            pattern: "started".to_string(),
            message: None,
        },
        OutputAssertion {
            pattern: "completed".to_string(),
            message: None,
        },
    ];
    let result = run_assertions("Job started and completed", &assertions);
    assert_eq!(result.len(), 0);

    let partial = run_assertions("Job started but crashed", &assertions);
    assert_eq!(partial.len(), 1); // "completed" not found
}
