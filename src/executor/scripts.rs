use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// File-based store for Rhai scripts used by script-type tasks.
#[derive(Clone)]
pub struct ScriptStore {
    dir: PathBuf,
}

/// Metadata about a stored script (name, type, size, last modified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub name: String,
    pub script_type: String,
    pub size: u64,
    pub modified: Option<String>,
}

/// A script with its full code content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptFull {
    pub name: String,
    pub script_type: String,
    pub code: String,
    pub size: u64,
}

const SCRIPT_EXTENSIONS: &[(&str, &str)] = &[("rhai", "rhai"), ("dockerfile", "dockerfile")];

impl ScriptStore {
    /// Creates a new script store, ensuring the directory exists.
    pub fn new(dir: &str) -> Result<Self, AppError> {
        let path = Path::new(dir);
        if !path.exists() {
            std::fs::create_dir_all(path)
                .map_err(|e| AppError::Internal(format!("failed to create scripts dir: {e}")))?;
        }
        Ok(Self {
            dir: path.to_path_buf(),
        })
    }

    /// Returns metadata for all scripts in the store directory.
    pub fn list(&self) -> Result<Vec<ScriptInfo>, AppError> {
        let mut scripts = Vec::new();
        let entries = std::fs::read_dir(&self.dir)
            .map_err(|e| AppError::Internal(format!("failed to read scripts dir: {e}")))?;
        for entry in entries {
            let entry = entry.map_err(|e| AppError::Internal(format!("{e}")))?;
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
            if let Some((_, script_type)) = SCRIPT_EXTENSIONS.iter().find(|(e, _)| *e == ext) {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let meta = std::fs::metadata(&path).ok();
                scripts.push(ScriptInfo {
                    name,
                    script_type: script_type.to_string(),
                    size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                    modified: meta.and_then(|m| m.modified().ok()).map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.to_rfc3339()
                    }),
                });
            }
        }
        scripts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(scripts)
    }

    /// Reads a script's full content by name (tries all extensions).
    pub fn get(&self, name: &str) -> Result<ScriptFull, AppError> {
        for (ext, script_type) in SCRIPT_EXTENSIONS {
            let path = self.dir.join(format!("{}.{}", name, ext));
            if path.exists() {
                let code = std::fs::read_to_string(&path)
                    .map_err(|e| AppError::Internal(format!("failed to read script: {e}")))?;
                let size = code.len() as u64;
                return Ok(ScriptFull {
                    name: name.to_string(),
                    script_type: script_type.to_string(),
                    code,
                    size,
                });
            }
        }
        Err(AppError::NotFound(format!("script '{}' not found", name)))
    }

    /// Saves a script by name and type after validating the name format.
    pub fn save(&self, name: &str, code: &str, script_type: Option<&str>) -> Result<(), AppError> {
        // Validate name (alphanumeric, dashes, underscores only)
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::BadRequest(
                "script name must be alphanumeric with dashes/underscores".into(),
            ));
        }
        let ext = match script_type {
            Some("dockerfile") => "dockerfile",
            _ => "rhai",
        };
        let path = self.dir.join(format!("{}.{}", name, ext));
        std::fs::write(&path, code)
            .map_err(|e| AppError::Internal(format!("failed to write script: {e}")))?;
        Ok(())
    }

    /// Deletes a script by name (tries all extensions).
    pub fn delete(&self, name: &str) -> Result<(), AppError> {
        for (ext, _) in SCRIPT_EXTENSIONS {
            let path = self.dir.join(format!("{}.{}", name, ext));
            if path.exists() {
                std::fs::remove_file(&path)
                    .map_err(|e| AppError::Internal(format!("failed to delete script: {e}")))?;
                return Ok(());
            }
        }
        Err(AppError::NotFound(format!("script '{}' not found", name)))
    }

    /// Reads just the code content of a script by name.
    pub fn read_code(&self, name: &str) -> Result<String, AppError> {
        self.get(name).map(|s| s.code)
    }
}
