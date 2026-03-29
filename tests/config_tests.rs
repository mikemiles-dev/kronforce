use std::sync::Mutex;
use std::time::Duration;

use kronforce::config::{AgentConfig, ControllerConfig};

// NOTE: from_env() reads real environment variables. Tests that mutate env vars
// must hold ENV_MUTEX to prevent races with other tests.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[test]
fn test_controller_config_default_db_path() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_DB").is_err() {
        let config = ControllerConfig::from_env();
        assert_eq!(config.db_path, "kronforce.db");
    }
}

#[test]
fn test_controller_config_default_bind_addr() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_BIND").is_err() {
        let config = ControllerConfig::from_env();
        assert_eq!(config.bind_addr, "0.0.0.0:8080");
    }
}

#[test]
fn test_controller_config_default_tick_interval() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_TICK_SECS").is_err() {
        let config = ControllerConfig::from_env();
        assert_eq!(config.tick_interval, Duration::from_secs(1));
    }
}

#[test]
fn test_controller_config_default_heartbeat_timeout() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_HEARTBEAT_TIMEOUT_SECS").is_err() {
        let config = ControllerConfig::from_env();
        assert_eq!(config.agent_heartbeat_timeout, Duration::from_secs(30));
    }
}

#[test]
fn test_controller_config_default_callback_url() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_CALLBACK_URL").is_err() && std::env::var("KRONFORCE_BIND").is_err()
    {
        let config = ControllerConfig::from_env();
        assert_eq!(config.callback_base_url, "http://0.0.0.0:8080");
    }
}

#[test]
fn test_controller_config_default_scripts_dir() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_SCRIPTS_DIR").is_err() {
        let config = ControllerConfig::from_env();
        assert_eq!(config.scripts_dir, "./scripts");
    }
}

#[test]
fn test_agent_config_default_bind_addr() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_AGENT_BIND").is_err() {
        let config = AgentConfig::from_env();
        assert_eq!(config.bind_addr, "0.0.0.0:8081");
    }
}

#[test]
fn test_agent_config_default_port() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_AGENT_BIND").is_err() {
        let config = AgentConfig::from_env();
        assert_eq!(config.port, 8081);
    }
}

#[test]
fn test_agent_config_default_controller_url() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_CONTROLLER_URL").is_err() {
        let config = AgentConfig::from_env();
        assert_eq!(config.controller_url, "http://localhost:8080");
    }
}

#[test]
fn test_agent_config_default_heartbeat_interval() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_HEARTBEAT_SECS").is_err() {
        let config = AgentConfig::from_env();
        assert_eq!(config.heartbeat_interval, Duration::from_secs(10));
    }
}

#[test]
fn test_agent_config_default_tags_empty() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_AGENT_TAGS").is_err() {
        let config = AgentConfig::from_env();
        assert!(config.tags.is_empty());
    }
}

#[test]
fn test_agent_config_default_agent_key_none() {
    let _lock = ENV_MUTEX.lock().unwrap();
    if std::env::var("KRONFORCE_AGENT_KEY").is_err() {
        let config = AgentConfig::from_env();
        assert!(config.agent_key.is_none());
    }
}

#[test]
fn test_controller_config_custom_env_vars() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("KRONFORCE_TICK_SECS", "5");
    }
    let config = ControllerConfig::from_env();
    assert_eq!(config.tick_interval, Duration::from_secs(5));
    unsafe {
        std::env::remove_var("KRONFORCE_TICK_SECS");
    }
}

#[test]
fn test_agent_config_custom_port_parsing() {
    let _lock = ENV_MUTEX.lock().unwrap();
    unsafe {
        std::env::set_var("KRONFORCE_AGENT_BIND", "0.0.0.0:9090");
    }
    let config = AgentConfig::from_env();
    assert_eq!(config.port, 9090);
    assert_eq!(config.bind_addr, "0.0.0.0:9090");
    unsafe {
        std::env::remove_var("KRONFORCE_AGENT_BIND");
    }
}
