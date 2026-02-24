use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

/// Represents a single entity (Player, NPC, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spawn {
    pub id: u32,
    pub name: String,
    pub level: u8,
    pub race: u16,
    pub class: u8,
    pub hp: u8,
    pub max_hp: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
    pub is_npc: bool,
}

/// The global state of the current zone
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneState {
    pub zone_id: u32,
    pub zone_name: String,
    pub my_name: String,            // Name of the player (for identification)
    pub my_spawn_id: Option<u32>,   // SpawnID of the player (once found)
    pub player: Option<Spawn>,
    pub spawns: HashMap<u32, Spawn>,
    pub profile: Option<crate::packets::PlayerProfile>,
}

/// Thread-safe wrapper for ZoneState
pub type SharedZoneState = Arc<Mutex<ZoneState>>;

impl ZoneState {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn set_my_name(&mut self, name: String) {
        self.my_name = name;
    }

    pub fn update_player_pos(&mut self, x: f32, y: f32, z: f32, heading: f32) {
        if let Some(player) = &mut self.player {
            player.x = x;
            player.y = y;
            player.z = z;
            player.heading = heading;
        }
    }

    pub fn add_or_update_spawn(&mut self, spawn: Spawn) {
        // Check if this spawn is ME
        if !self.my_name.is_empty() && spawn.name.eq_ignore_ascii_case(&self.my_name) {
             self.my_spawn_id = Some(spawn.id);
             // Also update player struct if we have one, or create it?
             // Usually PlayerProfile creates it. 
             // We can sync them.
             if let Some(player) = &mut self.player {
                 player.id = spawn.id;
                 player.x = spawn.x;
                 player.y = spawn.y;
                 player.z = spawn.z;
                 player.heading = spawn.heading;
             }
        }
        
        self.spawns.insert(spawn.id, spawn);
    }

    pub fn update_spawn_pos(&mut self, spawn_id: u32, x: f32, y: f32, z: f32, heading: f32) {
        // If this is ME, update player
        if let Some(my_id) = self.my_spawn_id {
            if spawn_id == my_id {
                self.update_player_pos(x, y, z, heading);
                // Also update the spawn entry for me?
                // `spawns` usually contains everyone including me.
            }
        }

        if let Some(spawn) = self.spawns.get_mut(&spawn_id) {
            spawn.x = x;
            spawn.y = y;
            spawn.z = z;
            spawn.heading = heading;
        }
    }

    pub fn remove_spawn(&mut self, id: u32) {
        self.spawns.remove(&id);
    }
}
