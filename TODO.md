# Kronforce TODO

Internal roadmap — not published in docs or README.

## Practical Gaps

- [x] Performance baseline documentation (docs/PERFORMANCE.md — limits, tuning, scaling strategies)
- [x] Migration guide from cron/Rundeck/Airflow (docs/MIGRATION.md + crontab import tool)
- [ ] v1.0 stable release (API/schema stability guarantees, semver commitment, CHANGELOG entry)

## Nice-to-Have

- [ ] Kubernetes Helm chart / operator
- [ ] OpenTelemetry / distributed tracing integration
- [x] Job templates / reusable workflow definitions (save as template + create from template)
- [ ] More granular RBAC (per-endpoint permissions beyond 4 fixed roles)
- [ ] Native Slack app with interactive approve/reject buttons (vs incoming webhook)
