//! The simulation tick: one step of the living world.
//!
//! Order of operations per tick:
//!   1. production at linked nodes
//!   2. caravan transfers along resource roads
//!   3. supply feeding forts
//!   4. settlement governance (food, happiness, order, growth)
//!   5. combat/siege resolution
//!   6. espionage
//!   7. influence / petitions
//!   8. bandits
//!   9. training
//!   10. AI
//!   11. victory checks
//!   5. espionage
//!   6. influence / petitions
//!   7. bandits
//!   8. AI
//!   9. victory checks

use super::ai;
use super::map::Map;
use super::types::*;
use super::world::{EventKind, Node, World};
use std::collections::HashSet;

impl World {
    /// Advance the world by one tick.
    pub fn tick(&mut self) {
        if self.victory.is_some() {
            return;
        }
        self.tick += 1;

        // Snapshot gold for King Rating / income display.
        self.player_ledger.prev_gold = self.player_ledger.gold;
        let player_gold_start = self.player_ledger.gold;
        let enemy_gold_start = self.enemy_ledger.gold;

        let player_linked = self.linked_set(Owner::Player);
        let enemy_linked = self.linked_set(Owner::Enemy);

        self.produce(&player_linked, &enemy_linked);
        self.caravans(&player_linked, &enemy_linked);
        self.supply_forts();
        self.governance(&player_linked, &enemy_linked);
        self.auto_employ_pass();
        self.general_upkeep_pass();
        self.combat();
        self.espionage();
        self.influence_pass();
        self.bandits();
        self.training_pass();
        ai::tick(self);

        // Income display (smoothed).
        let earned = self.player_ledger.gold - player_gold_start;
        if earned > 0 {
            self.stats.gold_earned += earned;
        }
        self.player_ledger.income_last = earned;
        self.player_ledger.income_shown =
            self.player_ledger.income_shown * 0.9 + earned as f32 * 0.1;
        let _ = enemy_gold_start;

        self.check_victory();

        // Periodic autosave into slot 0.
        super::save::maybe_autosave(self);
    }

    // -----------------------------------------------------------------
    // 1. Production
    // -----------------------------------------------------------------

    fn produce(&mut self, player_linked: &HashSet<u32>, enemy_linked: &HashSet<u32>) {
        let tick = self.tick;
        let player_mult = faction_def(self.player_faction).production
            * self.player_ledger.age.production_mult();
        let enemy_mult = faction_def(self.enemy_faction).production
            * self.enemy_ledger.age.production_mult();

        // Collect ids first to avoid borrow issues.
        let ids: Vec<u32> = self.nodes.keys().copied().collect();

        // Independent villages need a network of their own: without one their
        // own farms never produce and every neutral village on the map starves
        // to zero within a minute of play.
        let neutral_linked = self.linked_set(Owner::Neutral);

        for id in ids {
            let (owner, kind) = match self.nodes.get(&id) {
                Some(n) => (n.owner, n.kind),
                None => continue,
            };
            if owner == Owner::Bandit {
                continue;
            }

            // A node produces when its faction's network reaches it.
            // Neutral producers work when ANY faction links them (the
            // claim-by-connection idea from Seven Kingdoms), when they are
            // part of an independent village's own little network, or when
            // they are simply roaded to a village — an annexed village must
            // not lose the farm that used to feed it.
            let linked_ok = match owner {
                Owner::Player => player_linked.contains(&id),
                Owner::Enemy => enemy_linked.contains(&id),
                Owner::Neutral => {
                    player_linked.contains(&id)
                        || enemy_linked.contains(&id)
                        || neutral_linked.contains(&id)
                        || self.edges_of(id).any(|e| {
                            e.kind == EdgeType::Resource
                                && self
                                    .nodes
                                    .get(&e.other(id))
                                    .map(|n| n.is_settlement())
                                    .unwrap_or(false)
                        })
                }
                Owner::Bandit => false,
            };
            if !linked_ok {
                continue;
            }

            let mult = if owner == Owner::Player {
                player_mult
            } else if owner == Owner::Enemy {
                enemy_mult
            } else {
                1.0
            };

            let n = self.nodes.get_mut(&id).unwrap();
            let boosted = n.prod_mult() * mult;

            match kind {
                NodeType::Farm => {
                    // Base yield comfortably covers ~50 population so a
                    // single linked farm keeps a starter settlement fed.
                    let base = 5.0 + n.workers.iter().filter(|w| w.job == JobType::Farmer && w.ready()).count() as f32 * 1.5;
                    Self::accumulate(n, |s| &mut s.food, base * boosted);
                }
                NodeType::Woodcamp => {
                    let base = 2.0 + n.workers.iter().filter(|w| w.job == JobType::Lumberjack && w.ready()).count() as f32 * 1.5;
                    Self::accumulate(n, |s| &mut s.timber, base * boosted);
                }
                NodeType::Mine => {
                    let base = 2.0 + n.avg_skill(JobType::Miner) * 0.06;
                    Self::accumulate(n, |s| &mut s.ore, base * boosted);
                }
                NodeType::Factory => {
                    // Consumes ore + timber, emits goods.
                    if n.stock.ore > 0 && n.stock.timber > 0 {
                        let cycles = 1.0 * boosted;
                        let make = (cycles.floor() as i32).max(0);
                        let take = make.min(n.stock.ore).min(n.stock.timber);
                        if take > 0 {
                            n.stock.ore -= take;
                            n.stock.timber -= take;
                            n.stock.goods += take * 2;
                        }
                        // Fractional carry.
                        let frac = (cycles - cycles.floor()) as f32;
                        n.prod_frac += frac;
                        if n.prod_frac >= 1.0 && n.stock.ore > 0 && n.stock.timber > 0 {
                            n.stock.ore -= 1;
                            n.stock.timber -= 1;
                            n.stock.goods += 2;
                            n.prod_frac -= 1.0;
                        }
                    }
                }
                NodeType::Market => {
                    // Sells local goods for gold, scaled by trade skill.
                    let sell = n.stock.goods.min((2.0 * boosted) as i32 + 1);
                    if sell > 0 {
                        n.stock.goods -= sell;
                        let price = 10 * sell;
                        let mult = if owner == Owner::Player {
                            faction_def(self.player_faction).trade
                        } else if owner == Owner::Enemy {
                            faction_def(self.enemy_faction).trade
                        } else {
                            1.0
                        };
                        let gain = (price as f32 * mult) as i32;
                        match owner {
                            Owner::Enemy => self.enemy_ledger.gold += gain,
                            _ => self.player_ledger.gold += gain,
                        }
                        if owner == Owner::Player {
                            self.stats.gold_earned += gain;
                            self.stats.goods_sold += sell;
                        }
                    }
                }
                NodeType::Settlement => {
                    // Taxes: pop-driven, scaled by happiness.
                    let tax_f = n.population as f32 * 0.02 * (n.happiness / 70.0).clamp(0.4, 1.6);
                    n.prod_frac += tax_f;
                    if n.prod_frac >= 1.0 {
                        let tax = n.prod_frac.floor() as i32;
                        n.prod_frac -= tax as f32;
                        match owner {
                            Owner::Enemy => self.enemy_ledger.gold += tax,
                            _ => self.player_ledger.gold += tax,
                        }
                    }
                    // Research trickle comes from happiness & tier at capital-ish.
                }
                NodeType::Laboratory => {
                    // Research points, boosted by scholars.
                    let scholars = n
                        .workers
                        .iter()
                        .filter(|w| w.job == JobType::Scholar && w.ready())
                        .count() as f32;
                    let base = 1.0 + scholars * 1.2 + n.avg_skill(JobType::Scholar) * 0.04;
                    let pts = base * boosted;
                    match owner {
                        Owner::Enemy => self.enemy_ledger.tech += pts as i32,
                        _ => self.player_ledger.tech += pts as i32,
                    }
                }
                _ => {}
            }

            // Sabotage countdown.
            let n = self.nodes.get_mut(&id).unwrap();
            if n.sabotage > 0 {
                n.sabotage -= 1;
            }
        }
        let _ = tick;
    }

    /// Fractional accumulation helper for stock resources.
    fn accumulate(n: &mut super::world::Node, pick: fn(&mut Stock) -> &mut i32, amount: f32) {
        n.prod_frac += amount;
        if n.prod_frac >= 1.0 {
            let whole = n.prod_frac.floor() as i32;
            n.prod_frac -= whole as f32;
            *pick(&mut n.stock) += whole;
        }
    }

    // -----------------------------------------------------------------
    // 2. Caravans along resource roads
    // -----------------------------------------------------------------

    fn caravans(&mut self, _player_linked: &HashSet<u32>, _enemy_linked: &HashSet<u32>) {
        let mut transfers: Vec<(u32, u32, Owner)> = Vec::new();

        for edge in self.edges.iter_mut() {
            if edge.kind != EdgeType::Resource && edge.kind != EdgeType::Trade {
                continue;
            }
            if edge.caravans <= 0 {
                continue;
            }
            let len = edge.length();
            let period = (len * TILE_TRAVEL_TICKS / edge.caravans as f32).max(2.0);
            edge.progress += 1.0;
            if edge.progress >= period {
                edge.progress = 0.0;
                transfers.push((edge.from, edge.to, edge.by));
            }
        }

        for (a, b, by) in transfers {
            self.try_transfer(a, b, by);
        }
    }

    /// Move one unit of the most valuable transferable resource a→b or b→a.
    fn try_transfer(&mut self, a: u32, b: u32, by: Owner) {
        if self.move_stock(a, b, by) {
            return;
        }
        self.move_stock(b, a, by);
    }

    /// If `from` has something `to` wants, move it.
    fn move_stock(&mut self, from: u32, to: u32, by: Owner) -> bool {
        let (want_food, want_ore, want_timber, want_goods, from_owner, to_owner, from_kind, to_kind) = {
            let (f, t) = match (self.nodes.get(&from), self.nodes.get(&to)) {
                (Some(f), Some(t)) => (f, t),
                _ => return false,
            };
            let to_kind = t.kind;
            let want_food = to_kind == NodeType::Settlement;
            let want_ore = to_kind == NodeType::Factory || to_kind == NodeType::Market;
            let want_timber = to_kind == NodeType::Factory;
            let want_goods = matches!(to_kind, NodeType::Settlement | NodeType::Market);
            (
                want_food,
                want_ore,
                want_timber,
                want_goods,
                f.owner,
                t.owner,
                f.kind,
                to_kind,
            )
        };

        // Ownership gate: you only feed your own settlements/factories, or
        // neutral ones you're linked to (the 7K claim-by-connection idea).
        let transfer_ok = match to_owner {
            Owner::Player | Owner::Enemy => to_owner == from_owner || from_owner == Owner::Neutral,
            Owner::Neutral => true,
            Owner::Bandit => false,
        };
        if !transfer_ok {
            return false;
        }
        let _ = from_owner;

        // Food priority: a farm serves its hungriest neighbour first.
        //
        // Without this a passing empire's caravans can strip an independent
        // village's farm, and two settlements sharing one farm simply race —
        // the loser starving to zero population. Whatever else is going on,
        // the settlement closest to running out eats first.
        if from_kind == NodeType::Farm && to_kind == NodeType::Settlement {
            let shortfall = |s: &Node| (s.population / 10).max(1) * 15 - s.stock.food;
            let mut hungriest = to;
            let mut worst = self.nodes.get(&to).map(shortfall).unwrap_or(0);
            for e in self.edges_of(from) {
                if e.kind != EdgeType::Resource {
                    continue;
                }
                let other = e.other(from);
                if other == to {
                    continue;
                }
                if let Some(s) = self.nodes.get(&other) {
                    if !s.is_settlement() {
                        continue;
                    }
                    let d = shortfall(s);
                    if d > worst {
                        worst = d;
                        hungriest = other;
                    }
                }
            }
            if hungriest != to {
                return false;
            }
        }

        // Direction rules: producers feed consumers. Each caravan trip
        // ships the sender's whole stock of the mapped resource (minus a
        // reserve for settlements feeding other settlements).
        let (resource, keep): (fn(&mut Stock) -> &mut i32, i32) = {
            let f = &self.nodes[&from];
            let t_kind = to_kind;
            match (f.kind, t_kind) {
                (NodeType::Farm, NodeType::Settlement) if want_food => (|s| &mut s.food, 0),
                (NodeType::Mine, NodeType::Factory) if want_ore => (|s| &mut s.ore, 0),
                (NodeType::Mine, NodeType::Market) if want_ore => (|s| &mut s.ore, 0),
                (NodeType::Woodcamp, NodeType::Factory) if want_timber => {
                    (|s| &mut s.timber, 0)
                }
                (NodeType::Woodcamp, NodeType::Market) if want_timber => (|s| &mut s.timber, 0),
                (NodeType::Settlement, NodeType::Market) if want_goods => (|s| &mut s.goods, 0),
                (NodeType::Factory, NodeType::Settlement) if want_goods => (|s| &mut s.goods, 0),
                (NodeType::Factory, NodeType::Market) if want_goods => (|s| &mut s.goods, 0),
                // Settlement → Settlement: food sharing (deficit relief),
                // but the sender keeps 15 ticks of eating on hand.
                (NodeType::Settlement, NodeType::Settlement) if want_food => {
                    let keep = ((f.population / 10).max(1)) * 15;
                    (|s| &mut s.food, keep)
                }
                _ => return false,
            }
        };

        // How much does the sender have of the chosen resource?
        let have_amount = {
            let f = &self.nodes[&from];
            let s = &f.stock;
            match (f.kind, to_kind) {
                (NodeType::Farm, _) => s.food,
                (NodeType::Mine, _) => s.ore,
                (NodeType::Woodcamp, _) => s.timber,
                (NodeType::Factory, _) => s.goods,
                (NodeType::Settlement, NodeType::Market) => s.goods,
                (NodeType::Settlement, NodeType::Settlement) => s.food,
                _ => 0,
            }
        };

        let send = have_amount - keep;
        if send <= 0 {
            return false;
        }
        let is_goods = from_kind == NodeType::Factory;
        if let Some(f) = self.nodes.get_mut(&from) {
            *resource(&mut f.stock) -= send;
        }
        if let Some(t) = self.nodes.get_mut(&to) {
            *resource(&mut t.stock) += send;
        }
        if to_owner == Owner::Neutral && to_kind == NodeType::Settlement {
            self.award_fealty(to, by, is_goods);
        } else if from_owner == Owner::Neutral && from_kind == NodeType::Settlement {
            self.award_fealty(from, by, false);
        }
        true
    }

    /// Fealty accumulation & Rise of Legends commercial allegiance progression
    fn award_fealty(&mut self, settlement_id: u32, by: Owner, goods: bool) {
        if by != Owner::Player && by != Owner::Enemy {
            return;
        }
        let (name, prev_tier, new_tier) = {
            let Some(n) = self.nodes.get_mut(&settlement_id) else { return };
            if n.owner != Owner::Neutral || !n.is_settlement() {
                return;
            }
            let prev_tier = n.fealty_tier(by);
            let gain = if goods { 2.5 } else { 1.2 };
            let entry = n.fealty.entry(by).or_insert(0.0);
            *entry = (*entry + gain).min(100.0);

            // Tug of war: erode rival fealty
            let rival = if by == Owner::Player { Owner::Enemy } else { Owner::Player };
            if let Some(rf) = n.fealty.get_mut(&rival) {
                *rf = (*rf - gain * 0.2).max(0.0);
            }

            let new_tier = n.fealty_tier(by);
            (n.name.clone(), prev_tier, new_tier)
        };

        // Commercial dividend for Trading Partner+ (25+)
        if new_tier != AllegianceTier::WaryNeutral {
            match by {
                Owner::Player => self.player_ledger.gold += 3,
                Owner::Enemy => self.enemy_ledger.gold += 3,
                _ => {}
            }
        }

        if prev_tier != new_tier {
            match new_tier {
                AllegianceTier::TradingPartner => {
                    if by == Owner::Player {
                        self.log(
                            EventKind::Good,
                            format!("{} is now our Trading Partner! We receive commercial trade dividends.", name),
                        );
                    }
                }
                AllegianceTier::AlliedProtector => {
                    if by == Owner::Player {
                        self.log(
                            EventKind::Good,
                            format!("{} has recognized us as their Allied Protector! Their auxiliary levies stand ready.", name),
                        );
                    }
                }
                AllegianceTier::IntegratedDomain => {
                    if let Some(n) = self.nodes.get_mut(&settlement_id) {
                        n.owner = by;
                        if by == Owner::Player {
                            n.faction = Some(self.player_faction);
                        } else {
                            n.faction = Some(self.enemy_faction);
                        }
                    }
                    // Also flip connected neutral starter ring nodes
                    let ring_targets: Vec<u32> = self.edges.iter().filter_map(|e| {
                        if e.from == settlement_id {
                            Some(e.to)
                        } else if e.to == settlement_id {
                            Some(e.from)
                        } else {
                            None
                        }
                    }).collect();
                    for other in ring_targets {
                        if let Some(other_node) = self.nodes.get_mut(&other) {
                            if other_node.owner == Owner::Neutral && other_node.is_production() {
                                other_node.owner = by;
                                if by == Owner::Player {
                                    other_node.faction = Some(self.player_faction);
                                } else {
                                    other_node.faction = Some(self.enemy_faction);
                                }
                            }
                        }
                    }
                    match by {
                        Owner::Player => {
                            self.player_ledger.gold += 50;
                            self.stats.settlements_taken += 1;
                            self.log(
                                EventKind::Good,
                                format!("{} has sworn complete fealty and peacefully integrated into our realm!", name),
                            );
                        }
                        Owner::Enemy => {
                            self.enemy_ledger.gold += 50;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    /// Auto-employment: idle settlement population auto-fills connected production buildings.
    fn auto_employ_pass(&mut self) {
        if self.tick % AUTO_EMPLOY_INTERVAL != 0 {
            return;
        }

        let settlements: Vec<(u32, Owner)> = self
            .nodes
            .values()
            .filter(|n| n.is_settlement() && n.population > MIN_SETTLEMENT_POP_RESERVE)
            .map(|n| (n.id, n.owner))
            .collect();

        for (s_id, s_owner) in settlements {
            let current_pop = self.nodes.get(&s_id).map(|n| n.population).unwrap_or(0);
            if current_pop <= MIN_SETTLEMENT_POP_RESERVE {
                continue;
            }

            let mut queue = std::collections::VecDeque::new();
            let mut visited = HashSet::new();
            queue.push_back(s_id);
            visited.insert(s_id);
            let mut target_job_node = None;

            while let Some(curr) = queue.pop_front() {
                for e in self.edges.iter() {
                    if e.from != curr && e.to != curr {
                        continue;
                    }
                    if e.kind != EdgeType::Resource {
                        continue;
                    }
                    let next = if e.from == curr { e.to } else { e.from };
                    if visited.insert(next) {
                        if let Some(n) = self.nodes.get(&next) {
                            if n.owner == s_owner
                                && (n.is_production() || n.kind == NodeType::Laboratory)
                                && n.worker_count() < n.job_slots()
                            {
                                target_job_node = Some(next);
                                break;
                            }
                            if n.owner == s_owner {
                                queue.push_back(next);
                            }
                        }
                    }
                }
                if target_job_node.is_some() {
                    break;
                }
            }

            if let Some(target_id) = target_job_node {
                if let Some(s) = self.nodes.get_mut(&s_id) {
                    s.population -= 1;
                }
                if let Some(target) = self.nodes.get_mut(&target_id) {
                    let job = target.kind.job();
                    let name = super::map::agent_name(self.rng.next_u64());
                    target.workers.push(Worker::new(job, AUTO_EMPLOY_START_SKILL, 0, name));
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // 3. Supply: settlements feed their forts with garrison troops
    //    (followed by combat/siege resolution)
    // -----------------------------------------------------------------

    /// Feed garrisons from their villages along Supply roads.
    ///
    /// A fort holds at most a fifth of the people who feed it: the garrison
    /// and village settle at 80/20, so a village of 30 keeps 25 and its fort
    /// holds 5. When the village shrinks (raids, sieges, workers leaving) the
    /// excess marches home again, and a fort that loses its Supply road sends
    /// its troops back to the nearest friendly settlement.
    fn supply_forts(&mut self) {
        if self.tick % 6 != 0 {
            return;
        }

        // (settlement, fort) pairs reachable by a Supply road.
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for e in &self.edges {
            if e.kind != EdgeType::Supply {
                continue;
            }
            let (a, b) = match (self.nodes.get(&e.from), self.nodes.get(&e.to)) {
                (Some(a), Some(b)) => (a, b),
                _ => continue,
            };
            if a.owner != b.owner || a.owner == Owner::Neutral {
                continue;
            }
            if a.is_settlement() && b.kind == NodeType::Fort {
                pairs.push((a.id, b.id));
            } else if b.is_settlement() && a.kind == NodeType::Fort {
                pairs.push((b.id, a.id));
            }
        }

        for (settlement_id, fort_id) in pairs {
            let (s_pop, s_tier, f_pop) = match (self.nodes.get(&settlement_id), self.nodes.get(&fort_id)) {
                (Some(s), Some(f)) => (s.population, s.tier, f.population),
                _ => continue,
            };
            let cap = garrison_cap(s_tier, s_pop + f_pop);
            if f_pop < cap && s_pop > 1 {
                // One villager takes up the spear this cycle.
                if let Some(s) = self.nodes.get_mut(&settlement_id) {
                    s.population -= 1;
                }
                if let Some(f) = self.nodes.get_mut(&fort_id) {
                    f.population += 1;
                }
            } else if f_pop > cap && f_pop > 0 {
                // More spears than the village can support: send one home.
                if let Some(f) = self.nodes.get_mut(&fort_id) {
                    f.population -= 1;
                }
                if let Some(s) = self.nodes.get_mut(&settlement_id) {
                    s.population += 1;
                }
            }
        }

        // Orphaned garrisons: a fort with no Supply road is not being fed, so
        // its troops drift home to the nearest friendly settlement.
        if self.tick % 24 == 0 {
            let orphans: Vec<u32> = self
                .nodes
                .values()
                .filter(|n| {
                    n.kind == NodeType::Fort
                        && n.owner != Owner::Neutral
                        && n.owner != Owner::Bandit
                        && n.population > 0
                })
                .filter(|n| !self.edges_of(n.id).any(|e| e.kind == EdgeType::Supply))
                .map(|n| n.id)
                .collect();

            for fort_id in orphans {
                let (fx, fy, owner) = match self.nodes.get(&fort_id) {
                    Some(f) => (f.x, f.y, f.owner),
                    None => continue,
                };
                let home = self
                    .nodes
                    .values()
                    .filter(|n| n.owner == owner && n.is_settlement())
                    .min_by(|a, b| {
                        let da = Map::dist(a.x, a.y, fx, fy);
                        let db = Map::dist(b.x, b.y, fx, fy);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|n| n.id);
                if let Some(home) = home {
                    if let Some(f) = self.nodes.get_mut(&fort_id) {
                        f.population -= 1;
                    }
                    if let Some(s) = self.nodes.get_mut(&home) {
                        s.population += 1;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // 4. Governance: food, happiness, order, growth
    // -----------------------------------------------------------------

    fn governance(&mut self, _pl: &HashSet<u32>, _el: &HashSet<u32>) {
        let ids: Vec<u32> = self
            .nodes
            .values()
            .filter(|n| n.is_settlement())
            .map(|n| n.id)
            .collect();

        let mut messages: Vec<(EventKind, String)> = Vec::new();

        for id in ids {
            // ---- read phase (immutable) ----
            let (owner, food, pop, tier, loyalty, besieged, happiness, public_order, dominant_race, name) = {
                let n = match self.nodes.get(&id) {
                    Some(n) => n,
                    None => continue,
                };
                (
                    n.owner,
                    n.stock.food,
                    n.population,
                    n.tier,
                    n.loyalty,
                    n.besieged,
                    n.happiness,
                    n.public_order,
                    n.dominant_race,
                    n.name.clone(),
                )
            };

            let race = dominant_race.unwrap_or(Race::Human);
            let eat_f = (pop as f32 / 10.0) * race.food_consumption_multiplier();
            let eat = (eat_f.round() as i32).max(1);
            let starving = food < eat;

            // Employment across this owner's workable holdings. Settlements
            // cannot host a crew, so their slots must not count: including
            // them would cap a realm's employment below 100% forever.
            let (employed, posts) = self.employment_of(owner);
            let employment = if posts <= 0 {
                0.0
            } else {
                (employed as f32 / posts as f32 * 100.0).min(100.0)
            };

            let fort_order = if let Some(fid) = self.fort_supplying(id) {
                if let Some(fort) = self.nodes.get(&fid) {
                    if let Some(gid) = fort.general_id {
                        if let Some(gen) = self.general(gid) {
                            let harmony = if Some(gen.race) == dominant_race { 1.15 } else { 0.90 };
                            gen.fort_order_bonus() * harmony
                        } else {
                            ORDER_FORT_BONUS
                        }
                    } else {
                        ORDER_FORT_BONUS
                    }
                } else {
                    ORDER_FORT_BONUS
                }
            } else {
                0.0
            };

            let faction_happiness = match owner {
                Owner::Player => faction_def(self.player_faction).happiness as f32,
                Owner::Enemy => faction_def(self.enemy_faction).happiness as f32,
                _ => 0.0,
            };
            let growth_mult = match owner {
                Owner::Player => faction_def(self.player_faction).growth,
                Owner::Enemy => faction_def(self.enemy_faction).growth,
                _ => 0.6,
            } * race.growth_multiplier();

            // ---- compute phase (pure, no borrows of self) ----
            // Order is the civic backbone. It draws on a fed realm's civic
            // base, its working population, and the garrison keeping the
            // peace — nothing else. An undeveloped realm therefore sits
            // stable but stagnant rather than spiralling to zero.
            let civic = if starving { 0.0 } else { ORDER_CIVIC_BASE };
            let siege_penalty = if besieged { ORDER_SIEGE_PENALTY } else { 0.0 };
            let target_order = civic
                + employment * ORDER_EMPLOYMENT_WEIGHT
                + fort_order
                - siege_penalty;
            let new_public_order =
                (public_order + (target_order - public_order) * ORDER_PULL + ORDER_DRIFT)
                    .clamp(0.0, 100.0);

            let target_happy =
                (new_public_order * 0.6 + happiness * 0.4 + faction_happiness).clamp(0.0, 100.0);

            let (mut new_pop, new_food, mut new_happiness) = if starving {
                ((pop - 1).max(0), 0, (happiness - 4.0).max(0.0))
            } else {
                // Happiness follows order exactly — it has no optimistic drift
                // of its own. That is what makes the growth gate predictable
                // (idle 34/34, a lone fort 54/54) and keeps a garrison from
                // being enough on its own.
                (pop, food - eat, happiness)
            };
            if starving && owner == Owner::Player && self.tick % 50 == 0 {
                messages.push((EventKind::Bad, format!("{} is starving!", name)));
            }

            // Growth: one rng draw per settlement per tick.
            if !starving
                && new_happiness > GROWTH_MIN_HAPPINESS
                && new_public_order > GROWTH_MIN_ORDER
            {
                let chance = 0.02 * growth_mult * (new_happiness / 100.0 + 0.3);
                if self.rng.chance(chance) {
                    new_pop += 1;
                }
            } else if new_pop == 0 && (new_food >= eat * 10 || (new_food >= eat && self.fed_by_link(id)))
            {
                // Empty but well-stocked — or empty but still supplied by a
                // road: settlers drift in rather than leaving a dead node
                // forever. (A capital that starved before this could never
                // recover, which is how "cities with zero population" began.)
                if self.rng.chance(0.05) {
                    new_pop = 1;
                }
            }

            new_happiness += (target_happy - new_happiness) * 0.04;

            let new_tier = match new_pop {
                0..=99 => 0,
                100..=499 => 1,
                500..=1999 => 2,
                _ => 3,
            };

            let mut rebel = false;
            if loyalty < 35 && new_happiness < 30.0 && owner != Owner::Neutral {
                rebel = self.rng.chance(0.02);
            }

            // ---- write phase (single mutable borrow) ----
            if let Some(n) = self.nodes.get_mut(&id) {
                n.stock.food = new_food;
                n.population = new_pop;
                n.happiness = new_happiness;
                n.public_order = new_public_order;
                if new_tier > n.tier {
                    n.tier = new_tier;
                }
                if rebel {
                    n.owner = Owner::Neutral;
                    n.faction = None;
                    n.loyalty = 100;
                    n.happiness = 40.0;
                }
            }

            // ---- deferred logging (no borrows live) ----
            if new_tier > tier && owner == Owner::Player {
                let label = match new_tier {
                    1 => "town",
                    2 => "city",
                    3 => "metropolis",
                    _ => "village",
                };
                messages.push((
                    EventKind::Good,
                    format!("{} has grown into a {}", name, label),
                ));
            }
            if rebel {
                messages.push((
                    EventKind::War,
                    format!("{} has rebelled and declared independence!", name),
                ));
            }
        }

        for (kind, text) in messages {
            self.log(kind, text);
        }
    }

    /// Periodic general salary payment, loyalty drift, and mutiny check.
    fn general_upkeep_pass(&mut self) {
        if self.tick % 100 != 0 {
            return;
        }

        let mut revolts = Vec::new();
        let mut warnings = Vec::new();

        for gen in &mut self.generals {
            let salary = gen.salary;
            let (has_gold, ledger_gold) = match gen.owner {
                Owner::Player => (self.player_ledger.gold >= salary, &mut self.player_ledger.gold),
                Owner::Enemy => (self.enemy_ledger.gold >= salary, &mut self.enemy_ledger.gold),
                _ => (true, &mut self.player_ledger.gold),
            };

            if gen.owner == Owner::Player || gen.owner == Owner::Enemy {
                if has_gold {
                    *ledger_gold -= salary;
                    if gen.loyalty < 80 {
                        gen.loyalty = (gen.loyalty + 1).min(100);
                    }
                } else {
                    gen.loyalty = gen.loyalty.saturating_sub(6);
                    if gen.owner == Owner::Player {
                        warnings.push(format!(
                            "Treasury depleted: {} went unpaid and grew disgruntled!",
                            gen.name
                        ));
                    }
                }
            }

            if gen.loyalty < 20 && (gen.owner == Owner::Player || gen.owner == Owner::Enemy) {
                if gen.loyalty < 10 {
                    revolts.push((gen.id, gen.fort_id, gen.name.clone()));
                } else if gen.owner == Owner::Player && self.tick % 200 == 0 {
                    warnings.push(format!(
                        "CRITICAL: {} loyalty is dangerously low ({}%)! Risk of mutiny!",
                        gen.name, gen.loyalty
                    ));
                }
            }
        }

        for text in warnings {
            self.log(EventKind::Bad, text);
        }

        for (gid, fort_id_opt, name) in revolts {
            if let Some(fid) = fort_id_opt {
                let fort_name = if let Some(fort) = self.nodes.get_mut(&fid) {
                    fort.owner = Owner::Bandit;
                    fort.faction = None;
                    Some(fort.name.clone())
                } else {
                    None
                };
                if let Some(fname) = fort_name {
                    self.log(
                        EventKind::War,
                        format!(
                            "MUTINY! {} has rebelled and claimed {} as an independent stronghold!",
                            name, fname
                        ),
                    );
                }
            }
            if let Some(gen) = self.generals.iter_mut().find(|g| g.id == gid) {
                gen.owner = Owner::Bandit;
                gen.loyalty = 100;
            }
        }
    }

    // -----------------------------------------------------------------
    // 5. Combat: fort battles, sieges, bandit camps
    // -----------------------------------------------------------------

    fn combat(&mut self) {
        let combat_edges: Vec<(u32, u32, EdgeType)> = self
            .edges
            .iter()
            .filter(|e| matches!(e.kind, EdgeType::Combat | EdgeType::Siege))
            .map(|e| (e.from, e.to, e.kind))
            .collect();

        for (a, b, kind) in combat_edges {
            let (owner_a, owner_b, hostile) = {
                let (na, nb) = match (self.nodes.get(&a), self.nodes.get(&b)) {
                    (Some(x), Some(y)) => (x, y),
                    _ => continue,
                };
                (
                    na.owner,
                    nb.owner,
                    na.owner.is_hostile_to(nb.owner),
                )
            };
            if !hostile {
                continue;
            }

            match kind {
                EdgeType::Combat => self.resolve_fort_battle(a, b, owner_a, owner_b),
                EdgeType::Siege => self.resolve_siege(a, b, owner_a, owner_b),
                _ => {}
            }
        }

        // Clear besieged flags, re-set them below.
        for n in self.nodes.values_mut() {
            n.besieged = false;
        }
        for e in self.edges.iter().filter(|e| e.kind == EdgeType::Siege) {
            let target = if self.nodes.get(&e.from).map(|n| n.kind) == Some(NodeType::Fort) {
                e.to
            } else {
                e.from
            };
            if let Some(n) = self.nodes.get_mut(&target) {
                if n.is_settlement() {
                    n.besieged = true;
                }
            }
        }
    }

    fn resolve_fort_battle(&mut self, a: u32, b: u32, owner_a: Owner, owner_b: Owner) {
        let (
            garr_a,
            garr_b,
            skill_a,
            skill_b,
            faction_a,
            faction_b,
            race_a,
            race_b,
            equip_a,
            equip_b,
            gen_a,
            gen_b,
            is_camp,
        ) = {
            let (na, nb) = match (self.nodes.get(&a), self.nodes.get(&b)) {
                (Some(x), Some(y)) => (x, y),
                _ => return,
            };
            (
                na.population,
                nb.population,
                na.avg_skill(JobType::Soldier),
                nb.avg_skill(JobType::Soldier),
                na.faction,
                nb.faction,
                na.dominant_race.unwrap_or(Race::Human),
                nb.dominant_race.unwrap_or(Race::Human),
                na.equipment,
                nb.equipment,
                na.general_id
                    .and_then(|gid| self.general(gid).map(|g| (g.combat_multiplier(), g.id))),
                nb.general_id
                    .and_then(|gid| self.general(gid).map(|g| (g.combat_multiplier(), g.id))),
                na.kind == NodeType::BanditCamp || nb.kind == NodeType::BanditCamp,
            )
        };

        if garr_a <= 0 && garr_b <= 0 {
            return;
        }

        // Combat power: garrison + skills + faction/race/equipment/general bonuses + defense.
        let power = |garr: i32,
                     skill: f32,
                     faction: Option<Faction>,
                     race: Race,
                     equip: EquipmentTier,
                     gen: Option<(f32, u32)>,
                     is_defender: bool|
         -> f32 {
            let mut p = 1.0 + garr as f32 * 0.08 + skill * 0.02;
            if let Some(f) = faction {
                let d = faction_def(f);
                p *= d.combat;
            }
            if is_defender {
                p *= 1.15 * race.defense_multiplier() * equip.defense_mult();
            } else {
                p *= race.combat_multiplier() * equip.combat_mult();
            }
            if let Some((gen_mult, _)) = gen {
                p *= gen_mult;
            }
            p.max(0.1)
        };

        let atk_a = power(garr_a, skill_a, faction_a, race_a, equip_a, gen_a, false);
        let atk_b = power(garr_b, skill_b, faction_b, race_b, equip_b, gen_b, true);

        // Each side takes damage from the other's power.
        let dmg_to_b = (atk_a * 1.2).round() as i32;
        let dmg_to_a = (atk_b * 1.0).round() as i32;

        let mut a_falls = false;
        let mut b_falls = false;

        if let Some(n) = self.nodes.get_mut(&a) {
            n.population -= dmg_to_a;
            if n.population <= 0 {
                n.population = 0;
                a_falls = true;
            }
        }
        if let Some(n) = self.nodes.get_mut(&b) {
            n.population -= dmg_to_b;
            if n.population <= 0 {
                n.population = 0;
                b_falls = true;
            }
        }

        // Capture logic.
        if a_falls && !b_falls {
            let new_owner = if owner_b == Owner::Bandit { Owner::Neutral } else { owner_b };
            self.capture_node(a, new_owner, owner_b);
            if owner_b == Owner::Player {
                self.stats.battles_won += 1;
            }
            if let Some((_, gid)) = gen_b {
                if let Some(g) = self.general_mut(gid) {
                    g.kills += 1;
                }
            }
            if let Some((_, gid)) = gen_a {
                if let Some(g) = self.general_mut(gid) {
                    g.fort_id = None;
                    g.loyalty = g.loyalty.saturating_sub(25);
                }
            }
        } else if b_falls && !a_falls {
            let new_owner = if owner_a == Owner::Bandit { Owner::Neutral } else { owner_a };
            self.capture_node(b, new_owner, owner_a);
            if owner_a == Owner::Player {
                self.stats.battles_won += 1;
            }
            if let Some((_, gid)) = gen_a {
                if let Some(g) = self.general_mut(gid) {
                    g.kills += 1;
                }
            }
            if let Some((_, gid)) = gen_b {
                if let Some(g) = self.general_mut(gid) {
                    g.fort_id = None;
                    g.loyalty = g.loyalty.saturating_sub(25);
                }
            }
        }

        let _ = is_camp;
    }

    fn resolve_siege(&mut self, fort: u32, settlement: u32, fort_owner: Owner, _set_owner: Owner) {
        // Determine which end is the fort.
        let (fort_id, set_id) = {
            let a_is_fort = self.nodes.get(&fort).map(|n| n.kind == NodeType::Fort) == Some(true);
            if a_is_fort {
                (fort, settlement)
            } else {
                (settlement, fort)
            }
        };

        let (garr, def_bonus, set_pop, set_loyalty, set_faction, set_race, fort_equip, gen_mult) = {
            let f = match self.nodes.get(&fort_id) {
                Some(f) => f,
                None => return,
            };
            let s = match self.nodes.get(&set_id) {
                Some(s) => s,
                None => return,
            };
            (
                f.population,
                f.avg_skill(JobType::Soldier),
                s.population,
                s.loyalty,
                s.faction,
                s.dominant_race.unwrap_or(Race::Human),
                f.equipment,
                f.general_id
                    .and_then(|gid| self.general(gid).map(|g| g.combat_multiplier()))
                    .unwrap_or(1.0),
            )
        };
        if garr <= 0 {
            return;
        }

        // Siege damage vs defense (loyalty + tier + faction defense + race defense).
        let mut defense = 1.0 + set_loyalty as f32 * 0.01 + s_tier_bonus(self, set_id);
        if let Some(f) = set_faction {
            defense *= faction_def(f).defense;
        }
        defense *= set_race.defense_multiplier();

        let siege_power = (garr as f32 * 0.10 * (1.0 + def_bonus * 0.02) * fort_equip.combat_mult() * gen_mult).max(0.1);
        let damage = (siege_power / defense).round() as i32;

        let mut falls = false;
        if let Some(s) = self.nodes.get_mut(&set_id) {
            s.population -= damage.max(1);
            s.happiness = (s.happiness - 1.0).max(0.0);
            s.loyalty = (s.loyalty - 1).max(0);
            if s.population <= 0 {
                s.population = 0;
                falls = true;
            }
        }

        // Garrison attrition from urban resistance.
        if let Some(f) = self.nodes.get_mut(&fort_id) {
            f.population -= (set_pop / 15).max(1);
            if f.population < 0 {
                f.population = 0;
            }
        }

        if falls {
            let name = self
                .nodes
                .get(&set_id)
                .map(|n| n.name.clone())
                .unwrap_or_default();
            self.capture_node(set_id, fort_owner, fort_owner);
            // The garrison occupies: move some soldiers in.
            let occupy = self
                .nodes
                .get(&fort_id)
                .map(|f| (f.population / 2).min(20))
                .unwrap_or(0);
            if let Some(f) = self.nodes.get_mut(&fort_id) {
                f.population -= occupy;
            }
            if let Some(s) = self.nodes.get_mut(&set_id) {
                s.population = 10 + occupy;
                s.happiness = 35.0;
                s.public_order = 30.0;
                s.loyalty = 40;
            }
            if fort_owner == Owner::Player {
                self.stats.battles_won += 1;
                self.stats.settlements_taken += 1;
                self.log(EventKind::War, format!("You have captured {}!", name));
            } else if fort_owner == Owner::Enemy {
                self.log(EventKind::War, format!("{} has fallen to your enemy!", name));
            }
        }
    }

    /// Flip a node to a new owner; sever edges that no longer make sense
    /// and update labels.
    fn capture_node(&mut self, id: u32, new_owner: Owner, _capturer: Owner) {
        let old_owner = self.nodes.get(&id).map(|n| n.owner);
        let name = self.nodes.get(&id).map(|n| n.name.clone()).unwrap_or_default();
        let kind = self.nodes.get(&id).map(|n| n.kind);

        if let Some(n) = self.nodes.get_mut(&id) {
            n.owner = new_owner;
            n.faction = match new_owner {
                Owner::Player => Some(self.player_faction),
                Owner::Enemy => Some(self.enemy_faction),
                _ => None,
            };
            if n.kind == NodeType::BanditCamp && new_owner != Owner::Bandit {
                n.kind = NodeType::Settlement;
                n.name = place_name_new(self.seed ^ id as u64);
                n.population = 15;
                n.happiness = 40.0;
            }
        }

        // Border-gore cleanup: edges classified Supply/Combat that now
        // connect same-owner nodes are re-typed, hostile leftovers removed.
        let node_owner = new_owner;
        self.edges.retain(|e| {
            if e.from != id && e.to != id {
                return true;
            }
            let other = e.other(id);
            let other_owner = match self.nodes.get(&other) {
                Some(o) => o.owner,
                None => return false,
            };
            // Keep roads that still make sense.
            if node_owner == other_owner {
                true
            } else {
                // Hostile pair? keep only if it's a valid combat/siege/trade.
                node_owner.is_hostile_to(other_owner)
            }
        });

        if old_owner == Some(Owner::Player) && new_owner != Owner::Player {
            if kind == Some(NodeType::Settlement) {
                self.log(EventKind::War, format!("{} has been lost!", name));
            }
        }
    }

    // -----------------------------------------------------------------
    // 6. Espionage
    // -----------------------------------------------------------------

    fn espionage(&mut self) {
        let mut dead_spies: Vec<u32> = Vec::new();
        let mut events: Vec<(EventKind, String)> = Vec::new();

        // Travel progress.
        for spy in self.spies.iter_mut() {
            match spy.status {
                SpyStatus::Traveling | SpyStatus::Returning => {
                    let (from, to) = if spy.status == SpyStatus::Traveling {
                        (spy.home, spy.target)
                    } else {
                        (spy.target, spy.home)
                    };
                    let (fx, fy, tx, ty) = match (from, to) {
                        (f, t) => {
                            // Positions resolved lazily below via node lookup;
                            // store as 0 and fix with a second pass.
                            (f, t, f, t)
                        }
                    };
                    let _ = (fx, fy, tx, ty);
                    spy.progress += 1.0 / SPY_TRAVEL_TICKS_PER_TILE; // scaled by distance in pass below
                }
                SpyStatus::Embedded => {
                    spy.embed_ticks += 1;
                }
            }
        }

        // Distance-correct travel: recompute progress rate properly.
        for i in 0..self.spies.len() {
            let (status, home, target, progress) = {
                let s = &self.spies[i];
                (s.status, s.home, s.target, s.progress)
            };
            if status == SpyStatus::Embedded {
                continue;
            }
            let (from_id, to_id) = if status == SpyStatus::Traveling {
                (home, target)
            } else {
                (target, home)
            };
            let (fx, fy, tx, ty) = match (self.nodes.get(&from_id), self.nodes.get(&to_id)) {
                (Some(f), Some(t)) => (f.x, f.y, t.x, t.y),
                _ => {
                    // Node gone: spy is lost.
                    dead_spies.push(self.spies[i].id);
                    continue;
                }
            };
            let dist = Map::dist(fx, fy, tx, ty).max(1.0);
            let rate = 1.0 / (dist * SPY_TRAVEL_TICKS_PER_TILE);
            let spy = &mut self.spies[i];
            spy.progress += rate;
            if spy.progress >= 1.0 {
                if spy.status == SpyStatus::Traveling {
                    spy.status = SpyStatus::Embedded;
                    spy.progress = 0.0;
                    spy.embed_ticks = 0;
                } else {
                    // Arrived home: remove from active roster.
                    dead_spies.push(spy.id);
                }
            }
            let _ = progress;
        }

        // Embedded operations.
        let mut caught: Vec<u32> = Vec::new();
        for i in 0..self.spies.len() {
            let (owner, op, skill, target, embed) = {
                let s = &self.spies[i];
                if s.status != SpyStatus::Embedded {
                    continue;
                }
                (s.owner, s.op, s.skill, s.target, s.embed_ticks)
            };

            let Some(target_node) = self.nodes.get(&target) else {
                caught.push(self.spies[i].id); // target vanished
                continue;
            };
            if target_node.owner == owner {
                // Their own settlement: agent just goes dormant, recall later.
                continue;
            }

            let target_faction = target_node.faction;
            let target_name = target_node.name.clone();
            let target_loyalty = target_node.loyalty;

            // Detection risk per tick.
            let faction_spy_mult = match owner {
                Owner::Player => faction_def(self.player_faction).spy,
                _ => faction_def(self.enemy_faction).spy,
            };
            let counter = match target_faction {
                Some(f) => faction_def(f).spy,
                None => 1.0,
            };
            let base_risk = match op {
                SpyOp::Infiltrate => 0.008,
                SpyOp::Siphon => 0.018,
                SpyOp::Sabotage => 0.025,
                SpyOp::Incite => 0.022,
            };
            let risk = base_risk * counter / faction_spy_mult * (60.0 / (skill as f32 + 20.0));
            if self.rng.chance(risk as f32) {
                caught.push(self.spies[i].id);
                continue;
            }

            // Apply the operation.
            match op {
                SpyOp::Siphon => {
                    let skim = 1 + skill / 30 + (embed as i32) / 100;
                    let gain = skim.max(1);
                    // Steal from enemy treasury if target is theirs, else
                    // skimming the local stockpile.
                    if target_node.owner == Owner::Enemy {
                        self.enemy_ledger.gold = (self.enemy_ledger.gold - gain).max(0);
                    } else if let Some(t) = self.nodes.get_mut(&target) {
                        let available = t.stock.food.min(gain);
                        t.stock.food -= available;
                    }
                    if owner == Owner::Player {
                        self.player_ledger.gold += gain;
                        self.stats.gold_earned += gain;
                    } else if owner == Owner::Enemy {
                        self.enemy_ledger.gold += gain;
                    }
                    if let Some(s) = self.spies.get_mut(i) {
                        s.siphoned += gain;
                    }
                }
                SpyOp::Sabotage => {
                    if let Some(t) = self.nodes.get_mut(&target) {
                        t.sabotage = 40;
                        t.happiness = (t.happiness - 0.5).max(0.0);
                    }
                }
                SpyOp::Incite => {
                    if let Some(t) = self.nodes.get_mut(&target) {
                        t.loyalty = (t.loyalty - 1 - skill / 40).max(0);
                        t.happiness = (t.happiness - 0.7).max(0.0);
                        t.public_order = (t.public_order - 0.9).max(0.0);
                    }
                    // Rebellion at zero loyalty: defected to the spy's side.
                    if target_loyalty - 1 - skill / 40 <= 0 {
                        if self.rng.chance(0.35) {
                            let new_owner = owner;
                            let name = target_name.clone();
                            if let Some(t) = self.nodes.get_mut(&target) {
                                t.owner = new_owner;
                                t.faction = match new_owner {
                                    Owner::Player => Some(self.player_faction),
                                    Owner::Enemy => Some(self.enemy_faction),
                                    _ => None,
                                };
                                t.loyalty = 60;
                                t.happiness = 55.0;
                                t.population = (t.population / 2).max(10);
                            }
                            events.push((
                                EventKind::Spy,
                                format!("Your agents have turned {} against its rulers!", name),
                            ));
                            caught.push(self.spies[i].id); // mission done, exfil
                        }
                    }
                }
                SpyOp::Infiltrate => {
                    // Passive: slowly lifts loyalty doubts (small order wobble).
                    if let Some(t) = self.nodes.get_mut(&target) {
                        t.public_order = (t.public_order - 0.15).max(0.0);
                    }
                }
            }
        }

        // Process deaths.
        for id in caught {
            if let Some(idx) = self.spies.iter().position(|s| s.id == id) {
                let spy = self.spies.remove(idx);
                if spy.owner == Owner::Player {
                    self.stats.spies_caught += 1;
                    events.push((
                        EventKind::Spy,
                        format!("{} was captured in {}!", spy.name, self.node_name(spy.target)),
                    ));
                } else if spy.owner == Owner::Enemy {
                    events.push((
                        EventKind::Spy,
                        format!("You caught enemy agent {}!", spy.name),
                    ));
                }
            }
        }
        for id in dead_spies {
            self.spies.retain(|s| s.id != id);
        }

        for (k, t) in events {
            self.log(k, t);
        }
    }

    fn node_name(&self, id: u32) -> String {
        self.nodes.get(&id).map(|n| n.name.clone()).unwrap_or("somewhere".into())
    }

    // -----------------------------------------------------------------
    // 7. Influence & petitions
    // -----------------------------------------------------------------

    fn influence_pass(&mut self) {
        let neutrals: Vec<u32> = self
            .nodes
            .values()
            .filter(|n| n.owner == Owner::Neutral && n.is_settlement())
            .map(|n| n.id)
            .collect();

        for id in neutrals {
            // Both factions may petition a neutral.
            for viewer in [Owner::Player, Owner::Enemy] {
                let inf = self.influence(viewer, id);
                if inf >= 41.0 && self.rng.chance(0.01) {
                    let name = self.node_name(id);
                    if let Some(n) = self.nodes.get_mut(&id) {
                        n.owner = viewer;
                        n.loyalty = 70;
                        n.happiness = (n.happiness + 5.0).min(100.0);
                    }
                    if viewer == Owner::Player {
                        self.log(
                            EventKind::Good,
                            format!("{} has petitioned to join your empire!", name),
                        );
                    } else {
                        self.log(
                            EventKind::Bad,
                            format!("{} has sworn allegiance to your enemy", name),
                        );
                    }
                    break;
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // 8. Bandits
    // -----------------------------------------------------------------

    fn bandits(&mut self) {
        self.bandit_timer = self.bandit_timer.saturating_sub(1);
        if self.bandit_timer > 0 {
            return;
        }
        self.bandit_timer = 180 + (self.rng.next_u64() % 120) as u64;

        // Gather camps and their targets.
        let camps: Vec<(u32, f32, f32, i32)> = self
            .nodes
            .values()
            .filter(|n| n.kind == NodeType::BanditCamp && n.owner == Owner::Bandit)
            .map(|n| (n.id, n.x, n.y, n.population))
            .collect();

        for (camp_id, cx, cy, strength) in camps {
            // Grow the camp slowly.
            if let Some(c) = self.nodes.get_mut(&camp_id) {
                c.population += 3;
            }

            // Raid the nearest settlement within range.
            let mut target: Option<(u32, f32)> = None;
            for n in self.nodes.values() {
                if !n.is_settlement() || n.owner == Owner::Bandit {
                    continue;
                }
                let d = Map::dist(cx, cy, n.x, n.y);
                if d < 16.0 && target.map(|(_, bd)| d < bd).unwrap_or(true) {
                    target = Some((n.id, d));
                }
            }
            let Some((tid, _)) = target else { continue };

            let victim_owner = self.nodes.get(&tid).map(|n| n.owner);
            let victim_name = self.node_name(tid);
            let loot = 5 + strength / 4;

            if let Some(v) = self.nodes.get_mut(&tid) {
                let stolen = v.stock.food.min(loot);
                v.stock.food -= stolen;
                v.population = (v.population - 1).max(1);
                v.happiness = (v.happiness - 5.0).max(0.0);
            }

            match victim_owner {
                Some(Owner::Player) => {
                    let gold_steal = self.player_ledger.gold.min(loot / 2);
                    self.player_ledger.gold -= gold_steal;
                    self.log(
                        EventKind::Bad,
                        format!("Bandits raided {}! ({} gold, food stolen)", victim_name, gold_steal),
                    );
                }
                Some(Owner::Enemy) => {
                    self.enemy_ledger.gold = (self.enemy_ledger.gold - loot / 2).max(0);
                }
                Some(Owner::Neutral) => {
                    // Neutral raids weaken them → easier to annex.
                    if let Some(v) = self.nodes.get_mut(&tid) {
                        v.loyalty = (v.loyalty - 5).max(0);
                    }
                }
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------
    // 9. Training progression
    // -----------------------------------------------------------------

    fn training_pass(&mut self) {
        for n in self.nodes.values_mut() {
            for w in n.workers.iter_mut() {
                if w.training_left > 0 {
                    w.training_left -= 1;
                    if w.training_left == 0 {
                        w.skill = (w.skill + 1).min(100);
                    }
                } else {
                    // Slow on-the-job growth.
                    if w.skill < 100 && self.tick % 30 == 0 {
                        w.skill += 1;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // 10. Victory
    // -----------------------------------------------------------------

    fn check_victory(&mut self) {
        // Economic.
        if self.player_ledger.gold >= self.win_gold {
            self.victory = Some(Victory::Economic);
            return;
        }
        if self.enemy_ledger.gold >= self.win_gold {
            self.victory = Some(Victory::Defeat);
            return;
        }

        // Domination.
        let enemy_settlements = self.settlements_of(Owner::Enemy).len();
        let player_settlements = self.settlements_of(Owner::Player).len();

        if enemy_settlements == 0 && player_settlements > 0 {
            self.victory = Some(Victory::Domination);
            return;
        }
        if player_settlements == 0 {
            self.victory = Some(Victory::Defeat);
            return;
        }

        // The Silent Web.
        let (bound, total) = self.web_progress();
        if total > 0 && bound as f32 / total as f32 >= WEB_VICTORY_RATIO {
            self.victory = Some(Victory::Web);
        }
    }

    /// Compact progress description for the HUD.
    pub fn victory_progress(&self) -> String {
        let econ = self.player_ledger.gold.max(0) as f32 / self.win_gold as f32;
        let (bound, total) = self.web_progress();
        let dom_player = self.settlements_of(Owner::Player).len();
        let dom_total = dom_player + self.settlements_of(Owner::Enemy).len();
        format!(
            "Wealth {:.0}% · Domination {}/{} · Web {}/{}",
            econ * 100.0,
            dom_player,
            dom_total,
            bound,
            total
        )
    }
}

fn s_tier_bonus(w: &World, id: u32) -> f32 {
    w.nodes.get(&id).map(|n| n.tier as f32 * 2.0).unwrap_or(0.0)
}

fn place_name_new(seed: u64) -> String {
    super::map::place_name(seed)
}
