# Security policy

Devdeck handles shells, repositories, credentials, databases and third-party extensions. Report vulnerabilities privately; do not open a public issue for an unfixed vulnerability.

## Reporting

Use GitHub private vulnerability reporting on this repository, or email `security@hungpt99.dev`. Include affected version/commit, platform, reproduction steps, impact and a suggested mitigation. Redact real secrets.

## Response targets

- Acknowledgement within 72 hours.
- Severity assessment within 7 days.
- Critical fix target 30 days; high 60 days; lower severities in a planned release.
- Reporters are credited unless they prefer anonymity.

## Supported versions

Latest stable plus current beta/nightly. Older versions may receive backports.

## Built-in defenses

Deny-by-default capability permissions; approval prompts for dangerous actions; append-only hash-chained audit log with secret redaction; localhost-only authenticated MCP/API servers; signed extension archives with permission-diff on update; config migrations; local-first crash reports with no secrets or paths. See [docs/security/threat-model.md](docs/security/threat-model.md).
