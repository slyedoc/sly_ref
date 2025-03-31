use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::Prefab;

/// only used for init load, not updated currently
#[derive(Resource, Serialize, Deserialize, Reflect)]
#[reflect(Resource)]
pub struct RefConfig {
    pub prefabs: Vec<PrefabConfig>,
}

impl Default for RefConfig {
    fn default() -> Self {
        Self {
            prefabs: Vec::new(),
        }
    }
}
#[derive(Serialize, Deserialize, Reflect)]
pub struct PrefabConfig {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: f32,
    pub prefab: Prefab,
}
