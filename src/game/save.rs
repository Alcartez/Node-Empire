//! JSON save/load with slots + autosave, versioned so future schema
//! changes can migrate rather than break.

use super::world::World;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SAVE_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub world: World,
    pub saved_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSummary {
    pub slot: usize,
    pub saved_at: String,
    pub tick: u64,
    pub gold: i32,
    pub player_faction: String,
    pub victory_in_progress: bool,
}

fn save_dir() -> PathBuf {
    PathBuf::from("saves")
}

fn slot_path(slot: usize) -> PathBuf {
    if slot == 0 {
        save_dir().join("autosave.json")
    } else {
        save_dir().join(format!("save_slot_{}.json", slot))
    }
}

fn timestamp() -> String {
    // Format: HH:MM:SS UTC from system time (no chrono dependency, no local
    // timezone conversion).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs_of_day = now % 86400;
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    format!("{:02}:{:02}:{:02} UTC", h, m, s)
}

pub fn save(world: &World, slot: usize) -> Result<(), String> {
    let dir = save_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let file = SaveFile {
        version: SAVE_VERSION,
        world: world.clone(),
        saved_at: timestamp(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    std::fs::write(slot_path(slot), json).map_err(|e| e.to_string())
}

pub fn load(slot: usize) -> Result<World, String> {
    let path = slot_path(slot);
    let data = std::fs::read_to_string(&path).map_err(|_| "No save in that slot".to_string())?;
    let file: SaveFile = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    if file.version > SAVE_VERSION {
        return Err("Save is from a newer version".into());
    }
    Ok(file.world)
}

pub fn exists(slot: usize) -> bool {
    slot_path(slot).exists()
}

/// Describe what's in a slot, for the load menu.
pub fn summary(slot: usize) -> Option<SaveSummary> {
    let data = std::fs::read_to_string(slot_path(slot)).ok()?;
    let file: SaveFile = serde_json::from_str(&data).ok()?;
    Some(SaveSummary {
        slot,
        saved_at: file.saved_at,
        tick: file.world.tick,
        gold: file.world.player().gold,
        player_faction: super::types::faction_def(file.world.player_faction)
            .name
            .to_string(),
        victory_in_progress: false,
    })
}

/// Autosave cadence, called from the sim.
pub fn maybe_autosave(world: &World) {
    if !world.autosave {
        return;
    }
    if world.tick > 0 && world.tick.is_multiple_of(600) {
        let _ = save(world, 0);
    }
}
