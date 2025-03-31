mod actions;
use std::{f32::consts::PI, path::PathBuf};

pub use actions::*;
mod assets;
pub use assets::*;

mod comfy;
use bevy_enhanced_input::prelude::*;
use bevy_health_bar3d::prelude::*;

use bevy_tokio_tasks::TokioTasksPlugin;
pub use comfy::*;
mod save;
pub use save::*;
mod copy_paste;
pub use copy_paste::*;
mod select;
pub use select::*;
mod prefab;
pub use prefab::*;
mod ui;
pub use ui::*;
mod progress;
pub use progress::*;

use avian3d::prelude::*;
use bevy::{
    color::palettes::tailwind,
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
    math::vec3,
    prelude::*,
};
use bevy_prng::WyRand;
use bevy_rand::prelude::*;
//use rand::prelude::*;

fn main() {
    let file_path = config_file_path();
    let config = match std::fs::read(&file_path) {
        Ok(s) => ron::de::from_bytes::<RefConfig>(&s).unwrap_or_default(),
        Err(_) => {
            error!("Failed to load config file: {:?}", &file_path);
            RefConfig::default()
        }
    };

    let mut app = App::new();
    app.insert_resource(config)
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Ref".to_string(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    mode: AssetMode::Processed,
                    ..default()
                }),
                // .set(LogPlugin {
                //     filter: "info,wgpu_core=warn,wgpu_hal=warn,cosmic_text=warn,naga=warn".into(),
                //     level: Level::INFO,
                //     ..default()
                // }),
            bevy_inspector_egui::quick::WorldInspectorPlugin::new(),
            MeshPickingPlugin,
            EnhancedInputPlugin,
            PhysicsPlugins::default(), // using for collision detection
            //bevy_inspector_egui::quick::FilterQueryInspectorPlugin::<With<Selected>>::default(),
            
            TokioTasksPlugin::default(),
            HealthBarPlugin::<WorkflowProgress>::default(),
            EntropyPlugin::<WyRand>::default(),
            AppActionPlugin,        
        ))
        .insert_resource(
            ColorScheme::<WorkflowProgress>::new()
                .foreground_color(ForegroundColor::Static(tailwind::GRAY_200.into())),
        )
        .add_systems(
            Update,
            ui_select.run_if(|query: Query<Entity, With<Selected>>| !query.is_empty()),
        )
        .init_resource::<SaveTimer>()
        .add_event::<Save>()
        .add_event::<SpawnPrefab>()
        .add_systems(Startup, (setup, setup_ui))
        .add_systems(
            Update,
            (
                spawn_prefab.run_if(on_event::<SpawnPrefab>),
                update_progress,
                autosave,
                file_drop,

            ),
        )
        //.add_systems(PostUpdate, save_on_exit.run_if(on_event::<AppExit>))
        .add_systems(Last, save.run_if(on_event::<Save>))
        .register_type::<Prefab>()
        .register_type::<RefConfig>()
        .register_type::<PrefabConfig>()
        .register_type::<SaveTimer>()
        .register_type::<Save>()
        .run();
}

fn setup(
    mut commands: Commands,

    mut meshes: ResMut<Assets<Mesh>>,
    config: Res<RefConfig>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    //commands.spawn(InfiniteGridBundle::default());

    commands.spawn((
        Name::new("MainCamera"),
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: 0.1,
            ..default()
        },

        // Skybox {
        //     brightness: 5000.0,
        //     image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        //     ..default()
        // },
        // EnvironmentMapLight {
        //     diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
        //     specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        //     intensity: 2500.0,
        //     ..default()
        // },
        // movement
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    // directional 'sun' light
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
    ));

    // commands.spawn((
    //     PointLight {
    //         intensity: 20_000_000.,
    //         range: 500.0,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     Transform::from_xyz(4.0, 20.0, 4.0),
    // ));

    // Spawn the light.
    // commands.spawn((
    //     DirectionalLight {
    //         illuminance: 15000.0,
    //         shadows_enabled: true,
    //         ..default()
    //     },
    //     Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI * -0.15, PI * -0.15)),
    //     CascadeShadowConfigBuilder {
    //         maximum_distance: 3.0,
    //         first_cascade_far_bound: 0.9,
    //         ..default()
    //     }
    //     .build(),
    // ));

    // ground
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1000.0)))),
        Transform::from_translation(Vec3::new(0.0, 0.0, -10.0)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(tailwind::GREEN_900),
            metallic: 0.0,
            reflectance: 0.0,
            ..default()
        })),
    ));

    // add prefabs
    for (i, p) in config.prefabs.iter().enumerate() {
        commands.spawn((
            Transform::from_translation(vec3(p.translation.x, p.translation.y, i as f32 * 0.1)), // offset z so no z fighting
            Name::new(p.prefab.name.clone()),
            p.prefab.clone(),
        ));
    }
}

fn config_file_path() -> PathBuf {
    let root = std::env::var("BEVY_ASSET_ROOT").unwrap_or("".to_string());
    std::path::Path::new(&root).join("assets/ref/config.ron")
}

#[derive(Event)]
pub struct SpawnPrefab;

fn spawn_prefab(camera_transform: Single<&mut Transform, With<Camera>>, mut commands: Commands) {
    info!("Spawning prefab");
    let pos = camera_transform.translation + camera_transform.forward() * 4.0;
    commands.spawn((
        Name::new("Prefab"),
        Prefab {
            name: "Prefab".to_string(),
            workflow: Workflow::StaticImage { image: None },
        },
        Transform::from_translation(pos),
    ));
}
