<!-- Title: a conventional commit, e.g. `feat(api): …`, `fix(render): …`, `docs: …` -->

## Summary

<!-- What changes and why. Link the issue or spec (specs/NNN-…) if any. -->

## Test plan

- [ ] `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] OpenAPI contract regenerated if the generic API changed (`INKPDF_UPDATE_OPENAPI=1 cargo test -p inkpdf --test contract_openapi`)
- [ ] Documentation updated (`docs/src/…`, `cargo xtask docs generate`)
- [ ] Bundled packages verified if touched (`cargo xtask packages verify`)
