//! Enemy AI: a scored-priority agent that runs on a cadence tied to
//! difficulty. It builds economy, expands, wages war, and spies — enough
//! personality to feel like an opponent without cheating omniscience.

use super::map::Map;
use super::types::*;
use super::world::World;
use std::collections::HashSet;

pub fn tick(world: &mut World) {
    if world.victory.is_some() {
        return;
    }
    world.ai_timer = world.ai_timer.saturating_sub(1);
    if world.ai_timer > 0 {
        return;
    }
    world.ai_timer = world.difficulty.ai_interval();

    // Difficulty stipend (a gentle catch-up mechanism, disclosed in help).
    let bonus = world.difficulty.ai_income_bonus();
    if bonus > 0 {
        world.enemy_ledger.gold += bonus;
    }

    let linked = world.linked_set(Owner::Enemy);

    // Re-roll personality occasionally so the AI isn't static.
    if world.tick.is_multiple_of(600) {
        world.ai_personality = (world.rng.next_u64() % 3) as u8;
    }

    // Priority: don't starve → arm → expand economy → spy → conquer.
    if try_feed(world, &linked) {
        return;
    }
    if try_research(world) {
        return;
    }
    if try_defend(world, &linked) {
        return;
    }
    if try_offense(world, &linked) {
        return;
    }
    if try_expand(world, &linked) {
        return;
    }
    if try_spy(world) {
        return;
    }
    try_train(world, &linked);
}

// -------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------

fn enemy_settlements(world: &World) -> Vec<u32> {
    world
        .settlements_of(Owner::Enemy)
        .into_iter()
        .map(|n| n.id)
        .collect()
}

/// The AI's best neutral target by influence.
fn best_influence_target(world: &World) -> Option<(u32, f32)> {
    world
        .nodes
        .values()
        .filter(|n| n.owner == Owner::Neutral && n.is_settlement())
        .map(|n| (n.id, world.influence(Owner::Enemy, n.id)))
        .filter(|(_, inf)| *inf >= 41.0)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
}

// -------------------------------------------------------------------
// Behaviours
// -------------------------------------------------------------------

/// Keep the capital fed: build a farm if food is critically low.
fn try_feed(world: &mut World, linked: &HashSet<u32>) -> bool {
    let settlements = enemy_settlements(world);
    if settlements.is_empty() {
        return false;
    }

    // Is any settlement starving?
    let hungry = settlements.iter().any(|id| {
        world
            .node(*id)
            .map(|n| n.stock.food < 15 || n.happiness < 30.0)
            .unwrap_or(false)
    });
    if !hungry {
        return false;
    }

    // Find a farm spot near a settlement.
    for sid in &settlements {
        let Some(s) = world.node(*sid) else { continue };
        let (sx, sy) = (s.x, s.y);
        for attempt in 0..40 {
            let ang = world.rng.range_f(0.0, std::f32::consts::TAU);
            let r = world.rng.range_f(4.0, 9.0);
            let x = sx + ang.cos() * r;
            let y = sy + ang.sin() * r;
            let tx = x as i32;
            let ty = y as i32;
            if !world.map.in_bounds(tx, ty) || world.map.terrain_at(tx, ty) != Terrain::Grass {
                continue;
            }
            // Spacing check.
            let too_close = world
                .nodes
                .values()
                .any(|n| Map::dist(n.x, n.y, x, y) < 2.5);
            if too_close {
                continue;
            }
            if world.enemy_ledger.gold < build_cost(NodeType::Farm) {
                return false;
            }
            match world.place_building(NodeType::Farm, x, y, Owner::Enemy) {
                Ok(farm_id) => {
                    let _ = world.build_road(*sid, farm_id, Owner::Enemy);
                    return true;
                }
                Err(_) => {
                    let _ = attempt;
                    continue;
                }
            }
        }
        let _ = linked;
    }
    false
}

/// Advance the age when research is banked.
fn try_research(world: &mut World) -> bool {
    if let Some(cost) = world.enemy_ledger.age.cost_to_advance() {
        if world.enemy_ledger.tech >= cost {
            let _ = world.advance_age(Owner::Enemy);
            return true;
        }
    }
    false
}

/// Build a fort when an enemy fort/siege looms near our settlements.
fn try_defend(world: &mut World, _linked: &HashSet<u32>) -> bool {
    let settlements = enemy_settlements(world);
    if settlements.is_empty() {
        return false;
    }

    for sid in &settlements {
        let Some(s) = world.node(*sid) else { continue };
        let (sx, sy) = (s.x, s.y);

        // Threat: hostile fort or combat/siege edge within 14 tiles.
        let threatened = world.nodes.values().any(|n| {
            n.owner == Owner::Player
                && n.kind == NodeType::Fort
                && Map::dist(n.x, n.y, sx, sy) < 14.0
        }) || world
            .edges
            .iter()
            .any(|e| e.kind == EdgeType::Siege && (e.from == *sid || e.to == *sid));

        if !threatened {
            continue;
        }
        // Already have a fort defending this settlement?
        let has_fort = world.edges_of(*sid).any(|e| {
            e.kind == EdgeType::Supply
                && world
                    .nodes
                    .get(&e.other(*sid))
                    .map(|n| n.owner == Owner::Enemy && n.kind == NodeType::Fort)
                    .unwrap_or(false)
        });
        if has_fort {
            continue;
        }
        if world.enemy_ledger.gold < build_cost(NodeType::Fort) + 30 {
            continue;
        }

        // Place a fort between the settlement and the threat.
        for _ in 0..30 {
            let ang = world.rng.range_f(0.0, std::f32::consts::TAU);
            let r = world.rng.range_f(3.5, 7.0);
            let x = sx + ang.cos() * r;
            let y = sy + ang.sin() * r;
            let tx = x as i32;
            let ty = y as i32;
            if !world.map.in_bounds(tx, ty) || !world.map.terrain_at(tx, ty).buildable() {
                continue;
            }
            if world
                .nodes
                .values()
                .any(|n| Map::dist(n.x, n.y, x, y) < 3.0)
            {
                continue;
            }
            if let Ok(fort_id) = world.place_building(NodeType::Fort, x, y, Owner::Enemy) {
                let _ = world.build_road(*sid, fort_id, Owner::Enemy);
                return true;
            }
        }
    }
    false
}

/// Push toward the player: connect a fort to a hostile fort/settlement.
fn try_offense(world: &mut World, linked: &HashSet<u32>) -> bool {
    let aggression = world.difficulty.ai_aggression();
    let personality_aggro = if world.ai_personality == 1 { 1.4 } else { 1.0 };

    // Gather our forts with garrison.
    let forts: Vec<(u32, f32, f32, i32)> = world
        .nodes
        .values()
        .filter(|n| n.owner == Owner::Enemy && n.kind == NodeType::Fort && n.population > 6)
        .map(|n| (n.id, n.x, n.y, n.population))
        .filter(|(_, _, _, g)| (*g as f32) > 8.0 / (aggression * personality_aggro))
        .collect();

    for (fid, fx, fy, garrison) in forts {
        // Find a player target in range.
        let mut best: Option<(u32, f32)> = None;
        for n in world.nodes.values() {
            if n.owner != Owner::Player {
                continue;
            }
            let d = Map::dist(fx, fy, n.x, n.y);
            if d > MAX_EDGE_TILES || d < 1.5 {
                continue;
            }
            // Prefer weakly-held settlements, then forts.
            let score = if n.is_settlement() {
                d - (n.population as f32) * 0.02 - 5.0
            } else if n.kind == NodeType::Fort {
                d + 8.0 - n.population as f32 * 0.05
            } else {
                d
            };
            if best.map(|(_, bs)| score < bs).unwrap_or(true) {
                best = Some((n.id, score));
            }
        }

        if let Some((target, _)) = best {
            // Don't double-connect.
            if world.edge_between(fid, target).is_some() {
                // Reinforce instead: keep feeding the fort.
                let settlements = enemy_settlements(world);
                for sid in settlements {
                    if world.edge_between(sid, fid).is_none()
                        && world.build_road(sid, fid, Owner::Enemy).is_ok()
                    {
                        return true;
                    }
                }
                continue;
            }
            // Only attack if we have a reasonable troop advantage.
            let target_pop = world
                .node(target)
                .map(|n| {
                    if n.kind == NodeType::Fort || n.is_settlement() {
                        n.population
                    } else {
                        0
                    }
                })
                .unwrap_or(0);
            if garrison < target_pop && world.rng.chance(0.6) {
                continue;
            }
            if world.build_road(fid, target, Owner::Enemy).is_ok() {
                return true;
            }
        }
        let _ = linked;
    }
    false
}

/// Expand: new settlement or claim a neutral via influence.
fn try_expand(world: &mut World, _linked: &HashSet<u32>) -> bool {
    // Influence-based annexation is cheap — take it when available.
    if let Some((tid, inf)) = best_influence_target(world) {
        if inf >= 81.0 && world.enemy_ledger.gold >= 200 {
            if world.buy_settlement(tid, Owner::Enemy).is_ok() {
                return true;
            }
        } else if inf >= 61.0
            && world.enemy_ledger.gold >= 100
            && world.foment(tid, Owner::Enemy).is_ok()
        {
            return true;
        }
    }

    // Found a new settlement when flush with gold.
    let settlement_count = world.settlements_of(Owner::Enemy).len();
    let want_more = match world.ai_personality {
        1 => settlement_count < 4, // aggressive: forward bases
        2 => settlement_count < 5, // economic: more tax base
        _ => settlement_count < 3,
    };
    if !want_more || world.enemy_ledger.gold < 450 {
        return false;
    }

    let settlements = enemy_settlements(world);
    let Some(origin_id) = settlements.first().copied() else {
        return false;
    };
    let Some(origin) = world.node(origin_id) else {
        return false;
    };
    let (ox, oy) = (origin.x, origin.y);

    for _ in 0..50 {
        let ang = world.rng.range_f(0.0, std::f32::consts::TAU);
        let r = world.rng.range_f(7.0, 14.0);
        let x = ox + ang.cos() * r;
        let y = oy + ang.sin() * r;
        let tx = x as i32;
        let ty = y as i32;
        if !world.map.in_bounds(tx, ty) || world.map.terrain_at(tx, ty) != Terrain::Grass {
            continue;
        }
        if world
            .nodes
            .values()
            .any(|n| Map::dist(n.x, n.y, x, y) < 5.0)
        {
            continue;
        }
        if let Ok(new_id) = world.place_building(NodeType::Settlement, x, y, Owner::Enemy) {
            let _ = world.build_road(origin_id, new_id, Owner::Enemy);
            return true;
        }
    }
    false
}

/// Send a spy against the player (or steal tech via infiltration).
fn try_spy(world: &mut World) -> bool {
    if world.enemy_ledger.gold < 300 {
        return false;
    }
    // Only occasionally.
    if world.rng.chance(0.5) {
        return false;
    }

    // Already have enough spies active?
    let active = world
        .spies
        .iter()
        .filter(|s| s.owner == Owner::Enemy)
        .count();
    if active >= 2 {
        return false;
    }

    // Pick our biggest settlement, target player's biggest.
    let Some(home) = world
        .settlements_of(Owner::Enemy)
        .into_iter()
        .max_by_key(|n| n.population)
        .map(|n| n.id)
    else {
        return false;
    };
    let Some(target) = world
        .settlements_of(Owner::Player)
        .into_iter()
        .max_by_key(|n| n.population)
        .map(|n| n.id)
    else {
        return false;
    };

    if world.train_spy(home, Owner::Enemy).is_ok() {
        // Choose op by personality: aggressive incites, economic siphons.
        let op = match world.ai_personality {
            1 => SpyOp::Incite,
            2 => SpyOp::Siphon,
            _ => {
                if world.rng.chance(0.5) {
                    SpyOp::Siphon
                } else {
                    SpyOp::Incite
                }
            }
        };
        if let Some(spy) = world
            .spies
            .iter_mut()
            .find(|s| s.owner == Owner::Enemy && s.target == target)
        {
            spy.op = op;
        }
        return true;
    }
    false
}

/// Keep the workforce busy: train soldiers/scholars when idle.
fn try_train(world: &mut World, _linked: &HashSet<u32>) -> bool {
    if world.enemy_ledger.gold < 120 {
        return false;
    }
    if world.rng.chance(0.4) {
        return false;
    }

    // Prefer forts with room for soldiers; then labs for scholars.
    let forts: Vec<u32> = world
        .nodes
        .values()
        .filter(|n| n.owner == Owner::Enemy && n.kind == NodeType::Fort)
        .map(|n| n.id)
        .collect();

    for fid in forts {
        if let Some(f) = world.node(fid) {
            if f.worker_count() < f.job_slots() && world.train_worker(fid, 0, Owner::Enemy).is_ok()
            {
                return true;
            }
        }
    }

    // Build a lab if we have none and plenty of gold.
    let has_lab = world
        .nodes
        .values()
        .any(|n| n.owner == Owner::Enemy && n.kind == NodeType::Laboratory);
    if !has_lab && world.enemy_ledger.gold > 400 {
        if let Some(sid) = enemy_settlements(world).first().copied() {
            if let Some(s) = world.node(sid) {
                let (sx, sy) = (s.x, s.y);
                for _ in 0..30 {
                    let ang = world.rng.range_f(0.0, std::f32::consts::TAU);
                    let r = world.rng.range_f(3.5, 8.0);
                    let x = sx + ang.cos() * r;
                    let y = sy + ang.sin() * r;
                    let tx = x as i32;
                    let ty = y as i32;
                    if !world.map.in_bounds(tx, ty) || !world.map.terrain_at(tx, ty).buildable() {
                        continue;
                    }
                    if world
                        .nodes
                        .values()
                        .any(|n| Map::dist(n.x, n.y, x, y) < 3.0)
                    {
                        continue;
                    }
                    if let Ok(lab) = world.place_building(NodeType::Laboratory, x, y, Owner::Enemy)
                    {
                        let _ = world.build_road(sid, lab, Owner::Enemy);
                        return true;
                    }
                }
            }
        }
    }
    false
}
