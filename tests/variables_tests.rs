use chrono::Utc;
use std::collections::HashMap;

use kronforce::db::Db;
use kronforce::db::models::*;
use kronforce::executor::substitute_variables;

fn test_db() -> Db {
    let db = Db::open(":memory:").unwrap();
    db.migrate().unwrap();
    db
}

// --- DB CRUD Tests ---

#[test]
fn test_variable_create_and_get() {
    let db = test_db();
    let var = Variable {
        name: "API_HOST".to_string(),
        value: "https://api.example.com".to_string(),
        updated_at: Utc::now(),
        secret: false,
    };
    db.insert_variable(&var).unwrap();

    let fetched = db.get_variable("API_HOST").unwrap().unwrap();
    assert_eq!(fetched.name, "API_HOST");
    assert_eq!(fetched.value, "https://api.example.com");
}

#[test]
fn test_variable_get_nonexistent() {
    let db = test_db();
    assert!(db.get_variable("MISSING").unwrap().is_none());
}

#[test]
fn test_variable_list() {
    let db = test_db();
    db.insert_variable(&Variable {
        name: "B_VAR".to_string(),
        value: "b".to_string(),
        updated_at: Utc::now(),
        secret: false,
    })
    .unwrap();
    db.insert_variable(&Variable {
        name: "A_VAR".to_string(),
        value: "a".to_string(),
        updated_at: Utc::now(),
        secret: false,
    })
    .unwrap();

    let vars = db.list_variables().unwrap();
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0].name, "A_VAR"); // sorted by name
    assert_eq!(vars[1].name, "B_VAR");
}

#[test]
fn test_variable_update() {
    let db = test_db();
    db.insert_variable(&Variable {
        name: "HOST".to_string(),
        value: "old".to_string(),
        updated_at: Utc::now(),
        secret: false,
    })
    .unwrap();

    assert!(db.update_variable("HOST", "new").unwrap());
    let fetched = db.get_variable("HOST").unwrap().unwrap();
    assert_eq!(fetched.value, "new");
}

#[test]
fn test_variable_update_nonexistent() {
    let db = test_db();
    assert!(!db.update_variable("MISSING", "val").unwrap());
}

#[test]
fn test_variable_delete() {
    let db = test_db();
    db.insert_variable(&Variable {
        name: "TO_DELETE".to_string(),
        value: "val".to_string(),
        updated_at: Utc::now(),
        secret: false,
    })
    .unwrap();

    assert!(db.delete_variable("TO_DELETE").unwrap());
    assert!(db.get_variable("TO_DELETE").unwrap().is_none());
}

#[test]
fn test_variable_delete_nonexistent() {
    let db = test_db();
    assert!(!db.delete_variable("MISSING").unwrap());
}

#[test]
fn test_variable_upsert_insert() {
    let db = test_db();
    db.upsert_variable("NEW_VAR", "value1").unwrap();
    let fetched = db.get_variable("NEW_VAR").unwrap().unwrap();
    assert_eq!(fetched.value, "value1");
}

#[test]
fn test_variable_upsert_update() {
    let db = test_db();
    db.upsert_variable("MY_VAR", "value1").unwrap();
    db.upsert_variable("MY_VAR", "value2").unwrap();
    let fetched = db.get_variable("MY_VAR").unwrap().unwrap();
    assert_eq!(fetched.value, "value2");
}

#[test]
fn test_variable_get_all_map() {
    let db = test_db();
    db.upsert_variable("A", "1").unwrap();
    db.upsert_variable("B", "2").unwrap();
    let map = db.get_all_variables_map().unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get("A").unwrap(), "1");
    assert_eq!(map.get("B").unwrap(), "2");
}

// --- Substitution Tests ---

#[test]
fn test_substitute_single_variable() {
    let task = TaskType::Shell {
        command: "curl {{API_HOST}}/status".to_string(),
    };
    let mut vars = HashMap::new();
    vars.insert(
        "API_HOST".to_string(),
        "https://api.example.com".to_string(),
    );

    let result = substitute_variables(&task, &vars, None).unwrap();
    if let TaskType::Shell { command } = result {
        assert_eq!(command, "curl https://api.example.com/status");
    } else {
        panic!("expected Shell task type");
    }
}

#[test]
fn test_substitute_multiple_variables() {
    let task = TaskType::Shell {
        command: "curl {{HOST}}:{{PORT}}/api".to_string(),
    };
    let mut vars = HashMap::new();
    vars.insert("HOST".to_string(), "localhost".to_string());
    vars.insert("PORT".to_string(), "8080".to_string());

    let result = substitute_variables(&task, &vars, None).unwrap();
    if let TaskType::Shell { command } = result {
        assert_eq!(command, "curl localhost:8080/api");
    } else {
        panic!("expected Shell task type");
    }
}

#[test]
fn test_substitute_missing_variable_left_as_is() {
    let task = TaskType::Shell {
        command: "echo {{MISSING}}".to_string(),
    };
    let vars = HashMap::new();

    let result = substitute_variables(&task, &vars, None);
    assert!(result.is_none()); // no vars to substitute
}

#[test]
fn test_substitute_no_placeholders() {
    let task = TaskType::Shell {
        command: "echo hello".to_string(),
    };
    let mut vars = HashMap::new();
    vars.insert("UNUSED".to_string(), "value".to_string());

    let result = substitute_variables(&task, &vars, None);
    assert!(result.is_none()); // no {{ in the task
}

#[test]
fn test_substitute_special_json_characters() {
    let task = TaskType::Shell {
        command: "echo {{MSG}}".to_string(),
    };
    let mut vars = HashMap::new();
    vars.insert("MSG".to_string(), "he said \"hello\"".to_string());

    let result = substitute_variables(&task, &vars, None).unwrap();
    if let TaskType::Shell { command } = result {
        assert_eq!(command, "echo he said \"hello\"");
    } else {
        panic!("expected Shell task type");
    }
}

#[test]
fn test_substitute_empty_vars_map() {
    let task = TaskType::Shell {
        command: "echo {{VAR}}".to_string(),
    };
    let vars = HashMap::new();
    assert!(substitute_variables(&task, &vars, None).is_none());
}

#[test]
fn test_substitute_http_task() {
    let task = TaskType::Http {
        method: HttpMethod::Get,
        url: "{{BASE_URL}}/health".to_string(),
        body: None,
        headers: None,
        expect_status: None,
    };
    let mut vars = HashMap::new();
    vars.insert(
        "BASE_URL".to_string(),
        "https://api.example.com".to_string(),
    );

    let result = substitute_variables(&task, &vars, None).unwrap();
    if let TaskType::Http { url, .. } = result {
        assert_eq!(url, "https://api.example.com/health");
    } else {
        panic!("expected Http task type");
    }
}
