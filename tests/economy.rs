//! Settlement economy: public order, employment, the growth gates, and the
//! garrison split.
//!
//! These pin down the rules the HUD explains to the player, so that "why isn't
//! my village growing?" has a tested answer instead of a guess:
//!
//! * order = civic base + employment + fort supply (nothing else),
//! * growth needs happiness > 55 and order > 40,
//! * a fort holds at most a fifth of the settlement feeding it.

use node_empire::game::map::Map;
use node_empire::game::types::*;
use node_empire::game::world::{Node, World};

/// A controlled two-realm world: one player capital, one distant enemy
/// capital, and nothing else.
///
/// The enemy AI is parked (`ai_timer` never expires) and the enemy is poor, so
/// a growth measurement can't be disturbed by an opponent's decisions.
fn lab() -> World {
    let mut w = World::new(2026, Faction::SilkSpinners, Difficulty::Medium);
    w.nodes.clear();
    w.edges.clear();
    w.spies.clear();
    w.autosave = false;
    w.win_gold = i32::MAX; // no accidental economic victory
    w.ai_timer = u64::MAX; // the AI never takes a turn
    w.next_id = 100;

    let mut cap = Node::new(
        1,
        NodeType::Settlement,
        Owner::Player,
        Some(Faction::SilkSpinners),
        10.0,
        10.0,
        "Testholm".into(),
    );
    cap.stock.food = 100_000;
    w.nodes.insert(1, cap);

    let foe_faction = w.enemy_faction;
    let mut foe = Node::new(
        2,
        NodeType::Settlement,
        Owner::Enemy,
        Some(foe_faction),
        48.0,
        38.0,
        "Foehold".into(),
    );
    foe.stock.food = 100_000;
    w.nodes.insert(2, foe);

    w.enemy_ledger.gold = 0;
    w
}

fn mood(w: &mut World, id: u32, order: f32, happiness: f32) {
    if let Some(n) = w.node_mut(id) {
        n.public_order = order;
        n.happiness = happiness;
    }
}

/// A player-owned production building with `crew` ready novices posted.
fn works(w: &mut World, id: u32, kind: NodeType, crew: usize) {
    let faction = w.player_faction;
    let mut n = Node::new(
        id,
        kind,
        Owner::Player,
        Some(faction),
        10.0 + id as f32,
        16.0,
        format!("Works {id}"),
    );
    for i in 0..crew {
        n.workers
            .push(Worker::new(kind.job(), TRAIN_SKILLS[0], 0, format!("Hand {i}")));
    }
    w.nodes.insert(id, n);
    w.next_id = w.next_id.max(id + 1);
}

fn fort(w: &mut World, id: u32, garr: i32) {
    let faction = w.player_faction;
    let mut n = Node::new(
        id,
        NodeType::Fort,
        Owner::Player,
        Some(faction),
        16.0,
        10.0,
        format!("Fort {id}"),
    );
    n.population = garr;
    w.nodes.insert(id, n);
    w.next_id = w.next_id.max(id + 1);
}

/// Wire two nodes together with a straight geometric road (no terrain routing
/// is involved in these tests, so the network is exactly what we declare).
fn link(w: &mut World, from: u32, to: u32, kind: EdgeType) {
    let (ax, ay, bx, by) = {
        let a = &w.nodes[&from];
        let b = &w.nodes[&to];
        (a.x as i32, a.y as i32, b.x as i32, b.y as i32)
    };
    let id = w.next_id;
    w.next_id += 1;
    w.edges.push(Edge {
        id,
        from,
        to,
        kind,
        by: Owner::Player,
        caravans: 0,
        progress: 0.0,
        path: Map::straight_path((ax, ay), (bx, by)),
    });
}

fn pop(w: &World, id: u32) -> i32 {
    w.node(id).expect("node exists").population
}

/// Settlements cannot host a crew, so their slots must not enter the
/// denominator — that was what pinned every realm below the growth gate.
#[test]
fn settlement_slots_are_not_counted_as_employment() {
    let mut w = lab();
    assert_eq!(
        w.employment_of(Owner::Player),
        (0, 0),
        "a lone village has no workable posts"
    );

    works(&mut w, 10, NodeType::Farm, 1);
    assert_eq!(
        w.employment_of(Owner::Player),
        (1, 4),
        "only the farm's posts count"
    );
}

/// An idle realm is stable and stagnant: it does not grow, and it does not
/// spiral into unhappiness either.
#[test]
fn an_idle_realm_is_stable_but_stagnant() {
    let mut w = lab();
    works(&mut w, 10, NodeType::Farm, 0); // workable, but nobody works it
    mood(&mut w, 1, 40.0, 40.0); // sitting just under the growth gate

    let start = pop(&w, 1);
    for _ in 0..2000 {
        w.tick();
    }

    let cap = w.node(1).unwrap();
    assert_eq!(cap.population, start, "an undeveloped realm must not grow");
    assert!(
        (30.0..40.0).contains(&cap.public_order),
        "order settles at the civic base, got {:.1}",
        cap.public_order
    );
    assert!(
        cap.happiness < GROWTH_MIN_HAPPINESS,
        "happiness {:.1}",
        cap.happiness
    );
}

/// Filling posts is the whole point: 7 of 12 posts lifts order past the gate
/// and the settlement starts to grow.
#[test]
fn filling_posts_lifts_order_and_starts_growth() {
    let mut w = lab();
    works(&mut w, 10, NodeType::Farm, 3);
    works(&mut w, 11, NodeType::Woodcamp, 2);
    works(&mut w, 12, NodeType::Mine, 2);
    mood(&mut w, 1, 0.0, 0.0);

    let start = pop(&w, 1);
    for _ in 0..2000 {
        w.tick();
    }

    let cap = w.node(1).unwrap();
    assert!(cap.public_order > GROWTH_MIN_ORDER, "order {:.1}", cap.public_order);
    assert!(
        cap.happiness > GROWTH_MIN_HAPPINESS,
        "happiness {:.1}",
        cap.happiness
    );
    assert!(
        cap.population > start + 5,
        "a staffed realm should grow: {} -> {}",
        start,
        cap.population
    );
}

/// A fort alone lands exactly one point short of the growth gate (54 against
/// the required 55). That is deliberate: a garrison keeps the peace, but the
/// realm still needs working posts behind it.
#[test]
fn a_fort_alone_is_one_point_short_of_growth() {
    let mut w = lab();
    fort(&mut w, 20, 0);
    link(&mut w, 1, 20, EdgeType::Supply);
    mood(&mut w, 1, 0.0, 0.0);

    let start = pop(&w, 1);
    for _ in 0..2000 {
        w.tick();
    }

    let cap = w.node(1).unwrap();
    assert!(w.has_fort_supply(1), "the supply road should count");
    assert!(
        cap.public_order > 50.0 && cap.public_order < GROWTH_MIN_ORDER + 20.0,
        "order {:.1}",
        cap.public_order
    );
    assert!(
        cap.happiness > 50.0 && cap.happiness < GROWTH_MIN_HAPPINESS,
        "a fort alone must not be enough: happiness {:.1}",
        cap.happiness
    );

    // The village keeps at least four fifths of its people; the garrison may
    // never strip it.
    assert_eq!(
        pop(&w, 1) + pop(&w, 20),
        start,
        "feeding moves people, it does not create or destroy them"
    );
    assert!(
        pop(&w, 1) >= (start * 4) / 5,
        "village left with {} of {start}",
        pop(&w, 1)
    );
}

/// A fort *plus* a single staffed post is enough: the garrison buys order, the
/// worker buys the rest.
#[test]
fn a_fort_plus_one_staffed_post_reaches_growth() {
    let mut w = lab();
    works(&mut w, 10, NodeType::Farm, 1); // 1 of 4 posts = 25% employment
    fort(&mut w, 20, 0);
    link(&mut w, 1, 20, EdgeType::Supply);
    mood(&mut w, 1, 0.0, 0.0);

    let start = pop(&w, 1);
    for _ in 0..2000 {
        w.tick();
    }

    let cap = w.node(1).unwrap();
    assert!(cap.public_order > GROWTH_MIN_ORDER, "order {:.1}", cap.public_order);
    assert!(
        cap.happiness > GROWTH_MIN_HAPPINESS,
        "happiness {:.1}",
        cap.happiness
    );
    assert!(
        cap.population > start,
        "pop {} -> {}",
        start,
        cap.population
    );
}

/// A siege takes 25 order away, which holds a well-run realm under the gate.
#[test]
fn a_siege_holds_growth_down() {
    let mut w = lab();
    works(&mut w, 10, NodeType::Farm, 3);
    works(&mut w, 11, NodeType::Woodcamp, 2);
    works(&mut w, 12, NodeType::Mine, 2); // 7 of 12 staffed
    mood(&mut w, 1, 0.0, 0.0);

    // A hostile but empty fort next door: it cannot strike (no garrison) yet
    // it keeps the settlement officially under siege. Bandit-owned, so nothing
    // else in the sim has an opinion about it.
    let mut camp = Node::new(
        21,
        NodeType::Fort,
        Owner::Bandit,
        None,
        20.0,
        10.0,
        "Siegeworks".into(),
    );
    camp.population = 0;
    w.nodes.insert(21, camp);
    w.next_id = w.next_id.max(22);
    link(&mut w, 21, 1, EdgeType::Siege);

    let start = pop(&w, 1);
    for _ in 0..1500 {
        w.tick();
    }

    let cap = w.node(1).unwrap();
    assert!(cap.besieged, "the settlement should read as besieged");
    assert!(
        cap.happiness < GROWTH_MIN_HAPPINESS,
        "happiness {:.1}",
        cap.happiness
    );
    assert_eq!(cap.population, start, "a besieged town does not grow");
}

/// Starving zeroes the civic base: order collapses and people leave.
#[test]
fn starvation_collapses_civic_order() {
    let mut w = lab();
    works(&mut w, 10, NodeType::Farm, 0);
    mood(&mut w, 1, 60.0, 60.0);
    w.node_mut(1).unwrap().stock.food = 0;

    let start = pop(&w, 1);
    for _ in 0..200 {
        w.tick();
    }

    let cap = w.node(1).unwrap();
    assert!(cap.population < start, "a starving town loses people");
    assert!(
        cap.public_order < 20.0,
        "order collapses while starving: {:.1}",
        cap.public_order
    );
}

/// When a village shrinks (raids, workers leaving) the excess garrison is sent
/// home rather than drained out of the survivors.
#[test]
fn a_shrunken_village_calls_its_garrison_home() {
    let mut w = lab();
    fort(&mut w, 20, 0);
    link(&mut w, 1, 20, EdgeType::Supply);
    mood(&mut w, 1, 34.0, 34.0);

    for _ in 0..120 {
        w.tick();
    }
    let garr = pop(&w, 20);
    assert!(garr > 0, "the fort should have been reinforced");

    // A raid takes most of the village.
    w.node_mut(1).unwrap().population = 10;
    for _ in 0..120 {
        w.tick();
    }

    assert!(
        pop(&w, 20) < garr,
        "troops go home when there are fewer people to protect ({} -> {})",
        garr,
        pop(&w, 20)
    );
    assert!(pop(&w, 1) > 10, "and the village gets them back");
}

/// A fort that loses its Supply road is not being fed, so its garrison drifts
/// back to the nearest friendly settlement instead of sitting there forever.
#[test]
fn an_orphaned_garrison_marches_home() {
    let mut w = lab();
    fort(&mut w, 20, 5);
    mood(&mut w, 1, 34.0, 34.0);
    w.node_mut(1).unwrap().population = 25; // those five came from here

    for _ in 0..600 {
        w.tick();
    }

    assert_eq!(pop(&w, 20), 0, "an unfed garrison drifts home");
    assert_eq!(pop(&w, 1), 30, "and stands with its village again");
}

/// The tier ladder is a hard ceiling on top of the one-fifth share.
#[test]
fn a_garrison_is_capped_by_its_villages_tier() {
    assert_eq!(garrison_cap(0, 10_000), GARRISON_CEILING[0]);
    assert_eq!(garrison_cap(1, 10_000), GARRISON_CEILING[1]);
    assert_eq!(garrison_cap(2, 10_000), GARRISON_CEILING[2]);
    assert_eq!(garrison_cap(3, 10_000), GARRISON_CEILING[3]);
    assert!(
        GARRISON_CEILING.windows(2).all(|w| w[1] > w[0]),
        "the ceiling must rise with the tier"
    );
    // Even a hamlet keeps one spear at the gate.
    assert_eq!(garrison_cap(0, 1), 1);
    // 20% of the village that remains, not of the pool handed over.
    assert_eq!(garrison_cap(0, 30), 5);
    // …and the tier ceiling always wins over the share.
    assert_eq!(garrison_cap(1, 60), 10);
    assert_eq!(garrison_cap(0, 60), GARRISON_CEILING[0]);
}

/// Pass 1 added no required save fields, so a save written before it (without
/// the autosave flag) must still load, and must default to autosaving on.
#[test]
fn saves_from_before_the_autosave_flag_still_load() {
    let w = lab();
    let mut value = serde_json::to_value(&w).expect("world serialises");
    value
        .as_object_mut()
        .expect("world is a JSON object")
        .remove("autosave");
    assert!(value.get("autosave").is_none(), "field really removed");

    let text = serde_json::to_string(&value).unwrap();
    let loaded: World = serde_json::from_str(&text).expect("an older save must still load");
    assert!(loaded.autosave, "a missing flag defaults to autosaving on");
    assert_eq!(loaded.nodes.len(), w.nodes.len());
}

/// A real save from the repo, if present, must load and keep simulating.
#[test]
fn a_shipped_save_loads_and_keeps_running() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("saves/save_slot_1.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("no saves/save_slot_1.json present; nothing to check");
        return;
    };

    let file: node_empire::game::save::SaveFile =
        serde_json::from_str(&text).expect("a shipped save must load");
    let mut w = file.world;
    w.autosave = false;

    for _ in 0..300 {
        w.tick();
    }

    assert!(
        !w.settlements_of(Owner::Player).is_empty(),
        "the player must still hold a settlement"
    );
    for n in w.nodes.values() {
        assert!(n.population >= 0, "{} has negative population", n.name);
        assert!(
            n.stock.food >= 0 && n.stock.ore >= 0 && n.stock.timber >= 0 && n.stock.goods >= 0,
            "{} has negative stock",
            n.name
        );
        if n.is_settlement() {
            assert!(
                (0.0..=100.0).contains(&n.public_order),
                "{} order out of range: {}",
                n.name,
                n.public_order
            );
            assert!(
                (0.0..=100.0).contains(&n.happiness),
                "{} happiness out of range: {}",
                n.name,
                n.happiness
            );
        }
    }
}
