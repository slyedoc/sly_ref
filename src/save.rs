use bevy::prelude::*;

use crate::{Prefab, PrefabConfig, RefConfig};

/// A timer resource used to save the game state periodically.
#[derive(Debug, Resource, Deref, DerefMut, Reflect)]
#[reflect(Resource)]
pub struct SaveTimer(pub Timer);

impl Default for SaveTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(60., TimerMode::Repeating))
    }
}

#[derive(Event, Reflect)]
pub struct Save;

// save ever so often
pub fn autosave(mut commands: Commands, mut save_timer: ResMut<SaveTimer>, time: Res<Time>) {
    if save_timer.tick(time.delta()).just_finished() {
        info!("Autosaving...");
        commands.send_event(Save);
    }
}

// save when
pub fn save_on_exit(mut commands: Commands) {
    commands.send_event(Save);
}

// save the current state of the world
pub fn save(query: Query<(&Transform, &Prefab)>) {
    info!("Saving...");
    let mut config = RefConfig {
        prefabs: Vec::new(),
    };

    for (trans, prefab) in query.iter() {
        config.prefabs.push(PrefabConfig {

            prefab: prefab.clone(),
            translation: trans.translation,
            rotation: trans.rotation,
            scale: trans.scale.x,
        });
    }
    let prefab_count = config.prefabs.len();

    use ron::ser::{to_string_pretty, PrettyConfig};
    let pretty = PrettyConfig::new()
        .depth_limit(2)
        .separate_tuple_members(true)
        .enumerate_arrays(true);

    let s = to_string_pretty(&config, pretty).expect("Serialization failed");

    let root = std::env::var("BEVY_ASSET_ROOT").unwrap_or("".to_string());
    let file_path = std::path::Path::new(&root).join("assets/ref/config.ron");

    match std::fs::write(&file_path, s) {
        Ok(_) => {
            info!("Saved file: {:?} - {} images", &file_path, prefab_count);
        }
        Err(e) => {
            error_once!("Save failed: {:?}\n{:?}", &file_path, e);
        }
    };
}
