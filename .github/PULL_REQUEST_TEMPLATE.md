## Description
<!-- Provide a brief description of the changes introduced by this pull request. -->

## Related Issues
<!-- Link related issues, e.g. Closes #12, Fixes #34 -->

## Type of Change
- [ ] 🐛 Bug fix (non-breaking change which fixes an issue)
- [ ] ✨ New feature (non-breaking change which adds functionality)
- [ ] ⚖️ Game balance / Calibration change
- [ ] 🎨 UI / Visual polish
- [ ] 📖 Documentation update
- [ ] 🧪 Testing / Invariant addition

## Architectural Checklist
- [ ] Does this modify code in `src/game/`? If yes, confirm **zero Bevy engine imports** are introduced.
- [ ] Are all new serializable fields annotated with `#[serde(default)]` to preserve save-game backward compatibility?
- [ ] Have you run `cargo test` and ensured all simulation tests pass?
- [ ] Have you run `cargo fmt --check` and `cargo clippy`?
- [ ] If applicable, did you add an invariant test in `tests/`?
