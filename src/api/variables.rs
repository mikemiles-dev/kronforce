use axum::Json;
use axum::extract::{Path, State};
use chrono::Utc;
use serde::Deserialize;

use super::AppState;
use super::auth::AuthUser;
use crate::db::db_call;
use crate::db::models::Variable;
use crate::error::AppError;

/// Masks the value of secret variables for API responses.
fn mask_secret(var: Variable) -> Variable {
    if var.secret {
        Variable {
            value: "••••••••".to_string(),
            ..var
        }
    } else {
        var
    }
}

/// Returns all global variables (secret values are masked).
pub(crate) async fn list_variables(
    State(state): State<AppState>,
) -> Result<Json<Vec<Variable>>, AppError> {
    let vars = db_call(&state.db, move |db| db.list_variables()).await?;
    Ok(Json(vars.into_iter().map(mask_secret).collect()))
}

/// Returns a single variable by name (secret values are masked).
pub(crate) async fn get_variable(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Variable>, AppError> {
    let var = db_call(&state.db, move |db| db.get_variable(&name)).await?;
    match var {
        Some(v) => Ok(Json(mask_secret(v))),
        None => Err(AppError::NotFound("variable not found".into())),
    }
}

/// Validates that a variable name contains only alphanumeric characters and underscores.
fn validate_variable_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("variable name cannot be empty".into()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::BadRequest(
            "variable name must contain only alphanumeric characters and underscores".into(),
        ));
    }
    Ok(())
}

/// Request body for creating a new variable.
#[derive(Deserialize)]
pub(crate) struct CreateVariableRequest {
    name: String,
    value: String,
    #[serde(default)]
    secret: bool,
}

/// Checks that the authenticated user has write access.
fn require_write(auth: &AuthUser) -> Result<(), AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    Ok(())
}

/// Creates a new global variable after validating the name.
pub(crate) async fn create_variable(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateVariableRequest>,
) -> Result<(axum::http::StatusCode, Json<Variable>), AppError> {
    require_write(&auth)?;
    validate_variable_name(&req.name)?;

    let var = Variable {
        name: req.name,
        value: req.value,
        updated_at: Utc::now(),
        secret: req.secret,
    };
    let var_clone = var.clone();
    db_call(&state.db, move |db| db.insert_variable(&var_clone)).await?;

    let actor_key_id = auth.0.as_ref().map(|k| k.id);
    let actor_key_name = auth.0.as_ref().map(|k| k.name.clone());
    let var_name = var.name.clone();
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "variable.created",
            "variable",
            Some(&var_name),
            actor_key_id,
            actor_key_name.as_deref(),
            None,
        )
    })
    .await;

    Ok((axum::http::StatusCode::CREATED, Json(mask_secret(var))))
}

/// Request body for updating a variable's value.
#[derive(Deserialize)]
pub(crate) struct UpdateVariableRequest {
    value: String,
}

/// Updates an existing variable's value.
pub(crate) async fn update_variable(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
    Json(req): Json<UpdateVariableRequest>,
) -> Result<Json<Variable>, AppError> {
    require_write(&auth)?;
    let name_clone = name.clone();
    let value = req.value.clone();
    let updated = db_call(&state.db, move |db| db.update_variable(&name_clone, &value)).await?;
    if !updated {
        return Err(AppError::NotFound("variable not found".into()));
    }
    let actor_key_id = auth.0.as_ref().map(|k| k.id);
    let actor_key_name = auth.0.as_ref().map(|k| k.name.clone());
    let audit_name = name.clone();
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "variable.updated",
            "variable",
            Some(&audit_name),
            actor_key_id,
            actor_key_name.as_deref(),
            None,
        )
    })
    .await;

    let var = db_call(&state.db, move |db| db.get_variable(&name))
        .await?
        .ok_or_else(|| AppError::NotFound("variable not found".into()))?;
    Ok(Json(mask_secret(var)))
}

/// Deletes a variable by name.
pub(crate) async fn delete_variable(
    State(state): State<AppState>,
    Path(name): Path<String>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    require_write(&auth)?;
    let audit_name = name.clone();
    let deleted = db_call(&state.db, move |db| db.delete_variable(&name)).await?;
    if !deleted {
        return Err(AppError::NotFound("variable not found".into()));
    }

    let actor_key_id = auth.0.as_ref().map(|k| k.id);
    let actor_key_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "variable.deleted",
            "variable",
            Some(&audit_name),
            actor_key_id,
            actor_key_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
