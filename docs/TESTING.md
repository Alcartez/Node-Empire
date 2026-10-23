# Node Empire — Testing Guide

The simulation layer (`src/game/`) is pure Rust with no Bevy imports, so the
whole game can be tested headlessly. `cargo test` needs no window, GPU, or
assets.

## Commands

```bash
cargo test                                   # all suites
cargo test --test suite_fealty               # one suite
cargo test -- --nocapture                    # keep stdout
cargo fmt --check                            # CI gate
cargo clippy --all-targets -- -D warnings    # CI gate
```

CI runs all three on Windows and Linux (`.github/workflows/ci.yml`). Do not
quote a fixed test count in documentation; run `cargo test` and read the
summary.

## The harness (`tests/common/mod.rs`)

`ScenarioHarness` wraps a `World` with background interference switched off.

```rust
mod common;
use common::ScenarioHarness;
use node_empire::game::types::*;

#[test]
fn a_feature_works() {
    let mut harness = ScenarioHarness::new();
    let capital = harness.add_settlement(
        "Capital", Owner::Player, Some(Faction::Breplid), Race::Human, 100, 5000,
    );
    let fort = harness.add_fort(
        "Bastion", Owner::Player, Some(Faction::Breplid), Some(Race::Human),
        10, EquipmentTier::Bronze,
    );
    harness.connect(capital, fort, EdgeType::Supply, Owner::Player);
    harness.add_general("Lord Sterling", Owner::Player, Race::Human, Some(fort), 85, 70, 90);

    harness.step(100);
    harness.assert_invariants();
    harness.assert_serde_roundtrip();
}
```

### Builders and helpers

| Method | Purpose |
|---|---|
| `ScenarioHarness::new()` | blank sandbox; AI and autosave disabled |
| `ScenarioHarness::quiet()` | `new()` plus bandits disabled and dummy anchor/foe larders emptied, for deterministic influence/fealty tests |
| `add_settlement(name, owner, faction, race, pop, food)` | settlement fixture |
| `add_fort(name, owner, faction, race, garrison, equipment)` | fort fixture |
| `add_general(name, owner, race, fort, leadership, combat, loyalty)` | general fixture |
| `connect(from, to, kind, by)` | two-point road |
| `step(n)` / `step_until(max, predicate)` | advance the real tick pipeline |
| `assert_invariants()` | no NaN, no negative stock/pop, all edges and general pointers valid |
| `assert_serde_roundtrip()` | JSON round-trip fidelity |

A harness road has a two-point path, so `Edge::length()` is 2 and a single
caravan delivers every 4 ticks. Tests that care about pacing should use
`step_until` rather than assuming a delivery every 10 ticks.

## Suites

| File | Covers |
|---|---|
| `tests/economy.rs` | production, order/employment/growth gates, garrisons, save/load |
| `tests/spawn.rs` | starter rings, starter forts and generals, 200-seed viability |
| `tests/sim_stress.rs` | 6000-tick stability, supply caravan flow |
| `tests/seven_kingdoms_features.rs` | generals, equipment, race traits |
| `tests/suite_generals.rs` | salary deduction, unpaid loyalty decay, mutiny, leadership order bonus |
| `tests/suite_demographics.rs` | racial consumption and combat multipliers |
| `tests/suite_equipment.rs` | tier upgrades, cost, foreign-fort refusal |
| `tests/suite_invariants.rs` | procedural invariants across seeds, save round-trip |
| `tests/suite_auto_employ.rs` | one post per interval, no staffing without a road, reserve population, foreign property |
| `tests/suite_training.rs` | manual training cost, skill tier, population draw, 30-tick growth, refusal without charge |
| `tests/suite_fealty.rs` | delivery gain, Trading Partner dividend, rival erosion, integration and ring flip |

### Deliberately untested (tracked as 🟡 in `ROADMAP.md`)

- espionage operations and detection
- influence, petitions, foment, purchase
- combat resolution and sieges
- victory/defeat predicates
- AI decision making

Add a suite when you touch one of these; do not mark the system ✅ in the
README until you do.

## Rules for new tests

1. **Test the rule, not the constant.** Prefer `step_until` and state assertions
   over "after exactly N ticks".
2. **One behaviour per test.** Name it as a sentence:
   `a_fort_alone_is_one_point_short_of_growth`.
3. **Always end with `assert_invariants()`** when the test mutates world state.
4. **New serializable field?** Add its bounds to `assert_invariants()` and a
   save round-trip check.
5. **No sleeps, no real time.** Use `step(n)`; the sim is deterministic.
6. **No fixed test counts in prose.** Point at `cargo test`.
