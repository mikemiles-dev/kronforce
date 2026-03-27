# Release Process

## Versioning

Kronforce follows [Semantic Versioning](https://semver.org/):
- **MAJOR** — breaking changes (API, database schema incompatibilities)
- **MINOR** — new features, backward-compatible
- **PATCH** — bug fixes, backward-compatible

## Creating a Release

1. **Update version** in `Cargo.toml`:
   ```toml
   [package]
   version = "0.4.0"
   ```

2. **Update CHANGELOG** (if maintained) with release notes

3. **Commit and tag**:
   ```bash
   git add Cargo.toml
   git commit -m "release: v0.4.0"
   git tag v0.4.0
   git push origin main --tags
   ```

4. **GitHub Actions** automatically:
   - Builds binaries for Linux x86_64/ARM64 and macOS x86_64/ARM64
   - Generates SHA256 checksums
   - Creates a GitHub Release with auto-generated release notes
   - Attaches all binaries and checksums

## Pre-Release Checklist

- [ ] All CI checks pass (lint, test, migration check, build)
- [ ] `cargo fmt --all` — no formatting issues
- [ ] `cargo clippy --all-targets` — no warnings
- [ ] `cargo test --all` — all tests pass
- [ ] Migration tested: fresh DB creates successfully
- [ ] Migration tested: upgrade from previous version works
- [ ] Docker build succeeds: `docker compose -f docker-compose.full.yml build`
- [ ] Docker stack runs: controller starts, agent connects
- [ ] Manual smoke test: create job, trigger, verify execution
- [ ] Documentation is current (README, docs/, in-app docs)
- [ ] Version bumped in `Cargo.toml`

## Database Migrations

Migrations are versioned and applied automatically on startup. Each migration has:
- A version number (monotonically increasing integer)
- A description
- SQL statements

**Rules:**
- Never modify an existing migration — always add a new one
- New columns should be nullable or have defaults for backward compatibility
- Test migration path from the previous release tag

## Binary Artifacts

Each release includes:

| File | Description |
|---|---|
| `kronforce-linux-amd64` | Controller binary for Linux x86_64 |
| `kronforce-linux-arm64` | Controller binary for Linux ARM64 |
| `kronforce-darwin-amd64` | Controller binary for macOS x86_64 |
| `kronforce-darwin-arm64` | Controller binary for macOS ARM64 (Apple Silicon) |
| `kronforce-agent-linux-amd64` | Agent binary for Linux x86_64 |
| `kronforce-agent-linux-arm64` | Agent binary for Linux ARM64 |
| `kronforce-agent-darwin-amd64` | Agent binary for macOS x86_64 |
| `kronforce-agent-darwin-arm64` | Agent binary for macOS ARM64 |
| `checksums-sha256.txt` | SHA256 checksums for all binaries |

## Docker Images

Docker images are built from the `Dockerfile` in the repository root. Three compose configurations are provided:

- `docker-compose.yml` — controller only (production)
- `docker-compose.agent.yml` — agent only (production, separate machine)
- `docker-compose.full.yml` — controller + agent (local development)
