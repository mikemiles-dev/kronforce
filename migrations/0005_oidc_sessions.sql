-- version: 17
-- description: Add OIDC sessions and auth state tables

CREATE TABLE IF NOT EXISTS oidc_sessions (
    id_hash TEXT PRIMARY KEY,
    user_email TEXT NOT NULL,
    user_name TEXT NOT NULL,
    role TEXT NOT NULL,
    id_token_claims TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_active_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_oidc_sessions_expires ON oidc_sessions(expires_at);

-- Temporary OIDC state for CSRF protection during authorization code flow
CREATE TABLE IF NOT EXISTS oidc_auth_states (
    state TEXT PRIMARY KEY,
    nonce TEXT NOT NULL,
    created_at TEXT NOT NULL
);
