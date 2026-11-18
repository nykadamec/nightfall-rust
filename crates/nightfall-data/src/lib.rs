//! Shared data schemas and isometric projection helpers for Nightfall.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnemyDef {
    pub id: String,
    pub name: String,
    pub hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub xp_value: u32,
    pub sprite_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeaponDef {
    pub id: String,
    pub name: String,
    pub damage: f32,
    pub cooldown_s: f32,
    pub projectile_speed: f32,
    pub sprite_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaveDef {
    pub start_time_s: f32,
    pub duration_s: f32,
    pub spawn_interval_s: f32,
    pub enemy_ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ProjectData {
    pub enemies: Vec<EnemyDef>,
    pub weapons: Vec<WeaponDef>,
    pub waves: Vec<WaveDef>,
}

impl ProjectData {
    /// Load all JSON definitions from `<assets_dir>/data/`.
    pub fn load_from_dir(assets_dir: &Path) -> Result<Self> {
        let data_dir = assets_dir.join("data");
        let enemies = read_json_or_default(&data_dir.join("enemies.json"))?;
        let weapons = read_json_or_default(&data_dir.join("weapons.json"))?;
        let waves = read_json_or_default(&data_dir.join("waves.json"))?;
        Ok(Self {
            enemies,
            weapons,
            waves,
        })
    }

    /// Persist all JSON definitions to `<assets_dir>/data/`.
    pub fn save_to_dir(&self, assets_dir: &Path) -> Result<()> {
        let data_dir = assets_dir.join("data");
        fs::create_dir_all(&data_dir)
            .with_context(|| format!("Failed to create {}", data_dir.display()))?;
        write_json(&data_dir.join("enemies.json"), &self.enemies)?;
        write_json(&data_dir.join("weapons.json"), &self.weapons)?;
        write_json(&data_dir.join("waves.json"), &self.waves)?;
        Ok(())
    }

    /// Resolve the default `assets/` directory relative to the current working directory.
    pub fn default_assets_dir() -> PathBuf {
        PathBuf::from("assets")
    }
}

/// Convert 2D logical world coordinates into 2:1 isometric screen coordinates.
pub fn world_to_iso(wx: f32, wy: f32) -> (f32, f32) {
    let sx = wx - wy;
    let sy = (wx + wy) * 0.5;
    (sx, sy)
}

/// Convert 2:1 isometric screen coordinates back into logical world coordinates.
pub fn iso_to_world(sx: f32, sy: f32) -> (f32, f32) {
    let wx = sy + sx * 0.5;
    let wy = sy - sx * 0.5;
    (wx, wy)
}

fn read_json_or_default<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> Result<T> {
    if !path.exists() {
        return Ok(T::default());
    }
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("Invalid JSON in {}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let raw = serde_json::to_string_pretty(value)?;
    fs::write(path, raw).with_context(|| format!("Failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_roundtrip_is_accurate() {
        let (wx, wy) = (120.0, -45.0);
        let (sx, sy) = world_to_iso(wx, wy);
        let (rx, ry) = iso_to_world(sx, sy);
        assert!((rx - wx).abs() < 1e-4);
        assert!((ry - wy).abs() < 1e-4);
    }

    #[test]
    fn project_data_roundtrip_json() {
        let data = ProjectData {
            enemies: vec![EnemyDef {
                id: "imp".into(),
                name: "Imp".into(),
                hp: 25.0,
                speed: 100.0,
                damage: 8.0,
                xp_value: 4,
                sprite_path: "sprites/imp.png".into(),
            }],
            weapons: vec![],
            waves: vec![],
        };
        let encoded = serde_json::to_string(&data).unwrap();
        let decoded: ProjectData = serde_json::from_str(&encoded).unwrap();
        assert_eq!(data, decoded);
    }
}
