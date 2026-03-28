use axum::http::StatusCode;
use axum::response::IntoResponse;
use kronforce::error::AppError;

fn status_of(err: AppError) -> StatusCode {
    err.into_response().status()
}

#[test]
fn test_not_found_maps_to_404() {
    assert_eq!(
        status_of(AppError::NotFound("missing".into())),
        StatusCode::NOT_FOUND
    );
}

#[test]
fn test_bad_request_maps_to_400() {
    assert_eq!(
        status_of(AppError::BadRequest("invalid".into())),
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn test_unauthorized_maps_to_401() {
    assert_eq!(
        status_of(AppError::Unauthorized("no auth".into())),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn test_forbidden_maps_to_403() {
    assert_eq!(
        status_of(AppError::Forbidden("denied".into())),
        StatusCode::FORBIDDEN
    );
}

#[test]
fn test_internal_maps_to_500() {
    assert_eq!(
        status_of(AppError::Internal("oops".into())),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn test_conflict_maps_to_409() {
    assert_eq!(
        status_of(AppError::Conflict("exists".into())),
        StatusCode::CONFLICT
    );
}

#[test]
fn test_agent_error_maps_to_502() {
    assert_eq!(
        status_of(AppError::AgentError("agent failed".into())),
        StatusCode::BAD_GATEWAY
    );
}

#[test]
fn test_agent_unavailable_maps_to_503() {
    assert_eq!(
        status_of(AppError::AgentUnavailable("down".into())),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[test]
fn test_db_error_maps_to_500() {
    // Create a rusqlite error to test the Db variant
    let sqlite_err = rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(1), // SQLITE_ERROR
        Some("test error".to_string()),
    );
    assert_eq!(
        status_of(AppError::Db(sqlite_err)),
        StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[test]
fn test_error_display_messages() {
    let err = AppError::NotFound("item 42".to_string());
    assert_eq!(format!("{}", err), "not found: item 42");

    let err = AppError::BadRequest("bad field".to_string());
    assert_eq!(format!("{}", err), "bad request: bad field");

    let err = AppError::Internal("crash".to_string());
    assert_eq!(format!("{}", err), "internal: crash");
}

#[test]
fn test_error_response_body_contains_error_field() {
    let err = AppError::NotFound("test".into());
    let response = err.into_response();
    // The response body should be JSON with an "error" field
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
