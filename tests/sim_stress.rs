//! Long-run stability: 6000 ticks of simulation must not panic, the player
//! economy must grow, and the new Supply-edge garrison feeding must actually
//! put troops in forts (without it the AI could never attack).

use node_empire::game::types::*;
use node_empire::game::world::World;

#[test]
fn stress_6000_ticks() {
    let mut w = World::new(7, Faction::SilkSpinners, Difficulty::Medium);
    // Headless: never let a test clobber a real player's autosave slot.
    w.autosave = false;

    let gold0 = w.player().gold;
    let mut max_fort_pop = 0i32;
    let mut saw_combat_edge = false;

    // Give the player a garrisoned fort so the Supply feeding path runs for
    // the whole test (the AI rarely affords one early).
    let (sid, sx, sy) = w
        .settlements_of(Owner::Player)
        .first()
        .map(|n| (n.id, n.x, n.y))
        .expect("player has a starting settlement");
    let mut fort_id = None;
    'outer: for dy in -6..=6 {
        for dx in -6..=6 {
            let (fx, fy) = (sx as i32 + dx, sy as i32 + dy);
            if w.map.in_bounds(fx, fy) && w.map.terrain_at(fx, fy).buildable() {
                if w.can_place(NodeType::Fort, fx as f32, fy as f32, Owner::Player).is_ok() {
                    fort_id = Some(
                        w.place_building(NodeType::Fort, fx as f32, fy as f32, Owner::Player)
                            .expect("fort placed"),
                    );
                    break 'outer;
                }
            }
        }
    }
    let fort_id = fort_id.expect("fort spot near capital");
    w.build_road(sid, fort_id, Owner::Player)
        .expect("supply road built");

    for _ in 0..6000 {
        w.tick();
        for n in w.nodes.values() {
            if n.kind == NodeType::Fort {
                max_fort_pop = max_fort_pop.max(n.population);
            }
        }
        if w.edges.iter().any(|e| {
            matches!(e.kind, EdgeType::Combat | EdgeType::Siege)
        }) {
            saw_combat_edge = true;
        }
        if w.victory.is_some() {
            break;
        }
    }

    // Economy grew (or the game legitimately ended).
    if w.victory.is_none() {
        assert!(
            w.player().gold > gold0,
            "player gold shrank: {} -> {}",
            gold0,
            w.player().gold
        );
        assert!(w.settlements_of(Owner::Player).len() >= 1, "player lost all settlements");
    }

    // Fort garrisons are being fed via Supply edges over the long run.
    let fort_pop = w.node(fort_id).map(|n| n.population).unwrap_or(-1);
    assert!(
        max_fort_pop > 0 && fort_pop > 0,
        "garrison never fed: peak {max_fort_pop}, final {fort_pop}"
    );
    let _ = saw_combat_edge; // combat depends on AI choices; not guaranteed.

    // Save / load round-trip.
    let json = serde_json::to_string(&w).expect("serialize");
    let w2: World = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(w.tick, w2.tick);
    assert_eq!(w.nodes.len(), w2.nodes.len());
}

#[test]
fn supply_feeding_moves_pop_into_fort() {
    let mut w = World::new(99, Faction::SilkSpinners, Difficulty::Medium);
    w.autosave = false;

    // Force the scenario: pick a player settlement, build a fort adjacent to
    // it, connect with a Supply road, then tick.
    let settlement = w
        .settlements_of(Owner::Player)
        .first()
        .map(|n| (n.id, n.x, n.y, n.population))
        .expect("player has a starting settlement");
    let (sid, sx, sy, spop) = settlement;
    assert!(spop > 12, "starter settlement pop {spop} too low to feed forts");

    // Find a free tile within road range (14 tiles) that is buildable.
    let mut fort_pos = None;
    'outer: for dy in -6..=6 {
        for dx in -6..=6 {
            let (fx, fy) = (sx as i32 + dx, sy as i32 + dy);
            if w.map.in_bounds(fx, fy) && w.map.terrain_at(fx, fy).buildable() {
                if w.can_place(NodeType::Fort, fx as f32, fy as f32, Owner::Player).is_ok() {
                    fort_pos = Some((fx as f32, fy as f32));
                    break 'outer;
                }
            }
        }
    }
    let (fx, fy) = fort_pos.expect("found a fort spot near the capital");
    let fort_id = w
        .place_building(NodeType::Fort, fx, fy, Owner::Player)
        .expect("fort placed");
    w.build_road(sid, fort_id, Owner::Player)
        .expect("supply road built");

    // Let the feeding cadence (every 6 ticks) run for a few cycles.
    for _ in 0..60 {
        w.tick();
    }

    let fort_pop = w.node(fort_id).map(|n| n.population).unwrap_or(-1);
    let settle_pop = w.node(sid).map(|n| n.population).unwrap_or(-1);
    assert!(
        fort_pop > 0,
        "fort garrison stayed 0 after 60 ticks with a Supply road (settlement pop {settle_pop})"
    );
    assert!(
        settle_pop > 12,
        "settlement stripped bare: pop {settle_pop} (reserve is 12)"
    );
}
