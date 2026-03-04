# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.1.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public GitHub issue
2. Email security concerns to the maintainers via the repository contact
3. Include steps to reproduce and potential impact assessment
4. We aim to acknowledge reports within 48 hours

## Security Measures

- All dependencies audited daily via `cargo-deny` and `cargo-audit`
- `unsafe` code is forbidden project-wide (`unsafe_code = "forbid"`)
- BLAKE3 cryptographic hashing for all state integrity checks
- Supply chain security via pinned GitHub Actions SHAs and dependency allowlists
