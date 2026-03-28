use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// File-based store for Rhai scripts used by script-type tasks.
#[derive(Clone)]
pub struct ScriptStore {
    dir: PathBuf,
}

/// Metadata about a stored script (name, size, last modified).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub name: String,
    pub size: u64,
    pub modified: Option<String>,
}

/// A script with its full code content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptFull {
    pub name: String,
    pub code: String,
    pub size: u64,
}

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

    /// Returns metadata for all `.rhai` scripts in the store directory.
    pub fn list(&self) -> Result<Vec<ScriptInfo>, AppError> {
        let mut scripts = Vec::new();
        let entries = std::fs::read_dir(&self.dir)
            .map_err(|e| AppError::Internal(format!("failed to read scripts dir: {e}")))?;
        for entry in entries {
            let entry = entry.map_err(|e| AppError::Internal(format!("{e}")))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("rhai") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let meta = std::fs::metadata(&path).ok();
                scripts.push(ScriptInfo {
                    name,
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

    /// Reads a script's full content by name.
    pub fn get(&self, name: &str) -> Result<ScriptFull, AppError> {
        let path = self.script_path(name);
        if !path.exists() {
            return Err(AppError::NotFound(format!("script '{}' not found", name)));
        }
        let code = std::fs::read_to_string(&path)
            .map_err(|e| AppError::Internal(format!("failed to read script: {e}")))?;
        let size = code.len() as u64;
        Ok(ScriptFull {
            name: name.to_string(),
            code,
            size,
        })
    }

    /// Saves a script by name after validating the name format.
    pub fn save(&self, name: &str, code: &str) -> Result<(), AppError> {
        // Validate name (alphanumeric, dashes, underscores only)
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::BadRequest(
                "script name must be alphanumeric with dashes/underscores".into(),
            ));
        }
        let path = self.script_path(name);
        std::fs::write(&path, code)
            .map_err(|e| AppError::Internal(format!("failed to write script: {e}")))?;
        Ok(())
    }

    /// Deletes a script by name.
    pub fn delete(&self, name: &str) -> Result<(), AppError> {
        let path = self.script_path(name);
        if !path.exists() {
            return Err(AppError::NotFound(format!("script '{}' not found", name)));
        }
        std::fs::remove_file(&path)
            .map_err(|e| AppError::Internal(format!("failed to delete script: {e}")))?;
        Ok(())
    }

    /// Reads just the code content of a script by name.
    pub fn read_code(&self, name: &str) -> Result<String, AppError> {
        self.get(name).map(|s| s.code)
    }

    fn script_path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}.rhai", name))
    }
}
