Thank you for contributing to steinline!

We welcome bug reports, feature requests, and pull requests. Please follow these guidelines to make contributions easier to review and land.

Getting started

- Fork the repo and create a branch for your change: git checkout -b feat/your-feature
- Run tests and linters before opening a PR (see BUILD.md for setup).

PRs

- Provide a clear description and link to any related issue.
- Keep changes focused; one logical change per PR.
- Add tests for bug fixes and new features when possible.

Code style

- Rust: run cargo fmt and cargo clippy (warnings should be fixed)
- JS/TS: run npm run lint and npm run format

Testing

- Run unit tests: (cd src-tauri && cargo test)
- Run frontend tests: npm test

Reporting issues

- Use the issue templates when opening new issues (.github/ISSUE_TEMPLATE)

Thank you!
