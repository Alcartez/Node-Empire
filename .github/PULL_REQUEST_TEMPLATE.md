## Description

<!-- What changed, and why? -->

## Related issues

<!-- Closes #12, Fixes #34 -->

## Type of change

- [ ] Bug fix
- [ ] New feature
- [ ] Balance / calibration
- [ ] UI / visual polish
- [ ] Documentation
- [ ] Tests / invariants

## Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] Changes in `src/game/` introduce **no Bevy imports**
- [ ] New serializable fields use `#[serde(default)]` and have a load test
- [ ] New mechanics have a test in `tests/`
- [ ] `docs/MECHANICS.md` updated if a rule or constant changed
- [ ] `docs/PLAYTESTING.md` updated if the change needs manual verification
- [ ] No copied Seven Kingdoms code, data, or assets are included
