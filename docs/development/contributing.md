# Contributing

## Code Style

### Rust

- Follow `rustfmt` defaults
- No `unsafe_code` (forbidden in `Cargo.toml`)
- All clippy warnings must be resolved (`-D warnings`)
- MSRV: 1.75
- Use `tracing` for logging (not `println!`)
- Prefer `Result` over `panic!` for error handling

### TypeScript/Svelte

- Tabs for indentation
- Single quotes
- No trailing commas
- 100 character print width
- Strict TypeScript mode
- Use Svelte 5 runes syntax

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes
3. Run the full CI pipeline locally:
   ```bash
   mise run ci
   ```
4. Ensure all tests pass:
   ```bash
   mise run test
   mise run e2e
   ```
5. Ensure linting passes:
   ```bash
   mise run lint
   mise run format:check
   mise run check
   ```
6. Commit with a conventional commit message
7. Push and create a pull request

## Commit Messages

Follow conventional commits:

```
feat: add entity resolution
fix: correct z-score calculation
docs: update API reference
chore(release): bump version to v0.2.0
perf: optimize search query parsing
test: add backup/restore E2E tests
```

## Testing Requirements

- New features should include unit tests
- UI changes should include E2E tests
- Performance-critical code should include benchmarks

## Documentation

- Update relevant docs in `docs/` when changing functionality
- Update `SPEC.md` for requirement changes
- Update `CHANGELOG.md` for user-facing changes
