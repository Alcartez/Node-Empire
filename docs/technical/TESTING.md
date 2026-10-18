# Node Empire: Developer Testing Guide & Test Suite Architecture

This document explains the testing architecture and test suites in *Node Empire* (`node_empire`). It is designed for core developers, contributors, and open-source collaborators writing new game mechanics, demographic systems, or combat rules.

---

## 1. Architectural Philosophy: Headless & Deterministic

* **Zero Windowing/GPU in Game Layer**: All simulation logic (`src/game/`) is strictly separated from the presentation engine (`src/engine/`). Game logic must never depend on Bevy graphics, render pipelines, or windowing.
* **Serde State Fidelity**: Every game type (`World`, `Node`, `Edge`, `General`, `Race`, `EquipmentTier`, `Ledger`) must be fully serializable via `serde`.
* **Deterministic Stepping**: Tests run headlessly with `cargo test`. A 6,000-tick economy simulation executes in under 50 milliseconds.

---

## 2. Test Harness: `ScenarioHarness` (`tests/common/mod.rs`)

Writing ad-hoc setup code for every test can lead to fragile or noisy test files. The `ScenarioHarness` provides a fluent, isolated fixture builder:

```rust
use common::ScenarioHarness;
use node_empire::game::types::*;

#[test]
fn test_my_new_feature() {
    let mut harness = ScenarioHarness::new();
    
    // 1. Setup settlements and forts in 1 line:
    let capital_id = harness.add_settlement("Capital", Owner::Player, Some(Faction::Breplid), Race::Human, 100, 5000);
    let fort_id = harness.add_fort("Frontier Bastion", Owner::Player, Some(Faction::Breplid), Some(Race::Human), 10, EquipmentTier::Bronze);
    
    // 2. Connect road:
    harness.connect(capital_id, fort_id, EdgeType::Supply, Owner::Player);
    
    // 3. Station a General:
    let general_id = harness.add_general("Lord Sterling", Owner::Player, Race::Human, Some(fort_id), 85, 70, 90);
    
    // 4. Step simulation deterministically:
    harness.step(100);
    
    // 5. Assert invariants and save/load stability:
    harness.assert_invariants();
    harness.assert_serde_roundtrip();
}
```

### Key Capabilities of `ScenarioHarness`:
* **Isolated Environment**: Background enemy AI timers and random autosaves are deactivated so they do not interfere with deterministic measurements.
* **`step(ticks)`**: Advance simulation by exact tick counts.
* **`step_until(max_ticks, predicate)`**: Step until a condition is satisfied.
* **`assert_invariants()`**: Deeply validates the entire graph:
  - No `NaN` or out-of-range floats in coordinates, happiness, or civic order.
  - No negative populations or stockpiles.
  - All edges point to extant node IDs.
  - All assigned generals point to existing forts with matching IDs.
* **`assert_serde_roundtrip()`**: Serializes the world to JSON and back, asserting identical reconstruction.

---

## 3. Test Suites Directory Map

The test directory is structured into specialized feature suites:

| Test File | System Under Test | Description |
| :--- | :--- | :--- |
| **`tests/suite_generals.rs`** | Generals & Mutiny | Salary deduction, loyalty decay when broke, mutiny trigger ($< 10$ loyalty) into independent bandit fort, leadership order bonuses. |
| **`tests/suite_demographics.rs`** | Racial Demographics | Growth rates, cold-blooded vs ravenous food consumption (`Reptilian` vs `HiveSwarm`), martial multipliers. |
| **`tests/suite_equipment.rs`** | Armory & Equipment | Fort equipment tiers (`Militia` $\rightarrow$ `Bronze` $\rightarrow$ `IronClad` $\rightarrow$ `SteelSiege`), cost deductions, combat & siege defense buffs, foreign fort restrictions. |
| **`tests/suite_invariants.rs`** | Invariants & Procedural Map | Verifies world integrity, starting fort presence, and serde roundtrips across procedural random seeds. |
| **`tests/seven_kingdoms_features.rs`** | Core 7K Features | Starter Fort & Founding General spawning, general bribery/bonuses, equipment upgrades. |
| **`tests/economy.rs`** | Economy & Growth | Food starvation order collapse, employment gates for settlement growth, garrison population caps. |
| **`tests/sim_stress.rs`** | Performance & Stress | 6,000-tick continuous simulation stability and rapid supply caravan stress tests. |
| **`tests/spawn.rs`** | Procedural Capital Viability | Verifies starting rings and capitals across 200 procedural map seeds. |

---

## 4. Running the Test Suite

### Run All Tests:
```bash
cargo test
```

### Run a Specific Suite:
```bash
# Test Generals & Mutiny mechanics
cargo test --test suite_generals

# Test Racial Demographics & Consumption
cargo test --test suite_demographics

# Test Armory & Equipment Tiers
cargo test --test suite_equipment

# Test Invariants & Serde Roundtrip
cargo test --test suite_invariants
```

### Run with Verbose Output:
```bash
cargo test -- --nocapture
```

---

## 5. Guidelines for Future Contributors

1. **Always Add an Invariant Test**: When adding a new field to `Node`, `Edge`, `World`, or `General`, update `assert_invariants()` in `tests/common/mod.rs` to assert its validity bounds.
2. **Never Break Serialization**: Always test serde round-tripping (`harness.assert_serde_roundtrip()`) to ensure older save games and save slots remain stable.
3. **Keep Simulations Fast**: Avoid thread sleeps or real-time timeouts; use `harness.step(N)` to advance simulation ticks instantly.
