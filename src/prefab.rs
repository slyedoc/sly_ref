use std::fmt::{Display, Formatter};
use std::path::Path;
use std::time::{Duration, Instant};

use crate::{comfy, Selected, WorkflowProgress};
use bevy::ecs::component::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::pbr::NotShadowCaster;
use bevy::prelude::*;
use bevy_health_bar3d::prelude::*;
use bevy_inspector_egui::{prelude::ReflectInspectorOptions, InspectorOptions};
use bevy_prng::WyRand;
use bevy_rand::global::GlobalEntropy;
use bevy_tokio_tasks::{TaskContext, TokioTasksRuntime};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum::EnumIter;

// TODO: remove need for this
// hack to delay upating assets so reload works
const FILE_DELAY: f32 = 1.0;

#[derive(Component, Debug, Default, Clone, Reflect, Serialize, Deserialize, InspectorOptions)]
#[reflect(Component, InspectorOptions)]
#[require(BarSettings::<WorkflowProgress> = BarSettings::<WorkflowProgress> {
    offset: 1.5,
    width: 2.0,
    ..default()
})]
#[component(on_add = on_add_prefab)]
pub struct Prefab {
    pub name: String,
    pub workflow: Workflow,
}

#[derive(Component, EnumIter, PartialEq, Reflect, Debug, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub enum Workflow {
    StaticImage {
        image: Option<String>,
    },
    TextToImage {
        seed: u32,
        seed_random: bool,
        prompt: String,
        image: Option<String>,
    },
    TextToModel {
        seed: u32,
        seed_random: bool,
        prompt: String,
        num_faces: u32,
        image: Option<String>,
        model: Option<String>,
    },
}

impl Default for Workflow {
    fn default() -> Self {
        Workflow::StaticImage { image: None }
    }
}

impl Display for Workflow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Workflow::StaticImage { .. } => write!(f, "Static Image"),
            Workflow::TextToImage { .. } => write!(f, "Text To Image"),
            Workflow::TextToModel { .. } => write!(f, "Text To Model"),
        }
    }
}

pub fn on_add_prefab(mut world: DeferredWorld<'_>, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .entity(entity)
        .observe(on_drag)
        .observe(on_select)
        .observe(on_duplicate)
        .observe(on_delete)
        .observe(on_rename)
        .observe(on_generate)
        .observe(on_refresh_image)
        .observe(on_refresh_model);

    let prefab = world.entity(entity).get::<Prefab>().unwrap();

    let (image, model) = match &prefab.workflow {
        Workflow::TextToImage { image, .. } => (image.clone(), None),
        Workflow::TextToModel { image, model, .. } => (image.clone(), model.clone()),
        Workflow::StaticImage { image } => (image.clone(), None),
    };

    let asset_server = world.get_resource::<AssetServer>().unwrap();
    let image_handle: Handle<Image> = match image {
        Some(img) => asset_server.load(img),
        None => Handle::<Image>::default(),
    };
    let scene = if let Some(model) = model {
        Some(asset_server.load(GltfAssetLabel::Scene(0).from_asset(model)))
    } else {
        None
    };

    let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
    let mesh = meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(1.0)));

    let mut materials = world
        .get_resource_mut::<Assets<StandardMaterial>>()
        .unwrap();
    let mat = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    let e = world
        .commands()
        .entity(entity)
        .insert((NotShadowCaster, Mesh3d(mesh), MeshMaterial3d(mat)))
        .id();

    let model_entity = world
        .commands()
        .spawn((
            Name::new("Model"),
            Transform::from_translation(Vec3::new(0.0, -3.0, 0.0)),
            ChildOf { parent: e },
        ))
        .id();

    if let Some(scene) = scene {
        world
            .commands()
            .entity(model_entity)
            .insert(SceneRoot(scene));
    }
}

// move relative to camera
fn on_drag(
    drag: Trigger<Pointer<Drag>>,
    mut transforms: Query<&mut Transform, (Without<Camera>, With<Prefab>)>,
    camera_transforms: Single<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    if let Ok(mut transform) = transforms.get_mut(drag.target) {
        let scale = camera_transforms
            .translation
            .distance(transform.translation)
            * 0.1;

        let movement = Vec3::new(
            drag.delta.x * time.delta_secs(),
            -drag.delta.y * time.delta_secs(),
            0.0,
        );

        transform.translation += camera_transforms.rotation * movement * scale;
    }
}

// this could add a event instead of updateing
fn on_select(
    target: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut query: Query<(Entity, Option<&Selected>), With<Prefab>>,
) {
    for (e, selected) in query.iter_mut() {
        if e == target.target {
            if selected.is_none() {
                commands.entity(e).insert(Selected);
            }
        } else {
            if selected.is_some() {
                commands.entity(e)
                .remove::<Selected>();
            }
        }
    }
}

#[derive(Event)]
pub struct Duplicate;

fn on_duplicate(
    trigger: Trigger<Duplicate>,
    mut commands: Commands,
    query: Query<(&Prefab, &Transform)>,
) {
    let entity = trigger.target();

    let (prefab, trans) = query.get(entity).unwrap();

    // find new name
    let names = query
        .iter()
        .map(|(p, _)| p.name.clone())
        .collect::<Vec<_>>();

    let new_name = create_unique_name(&prefab.name, names);

    let mut new_prefab = prefab.clone();
    new_prefab.name = new_name.clone();

    match &mut new_prefab.workflow {
        Workflow::StaticImage { image } => {
            if let Some(img) = image {
                *img = copy_asset(&img, &new_prefab.name);
            }
        }
        Workflow::TextToImage { image, .. } => {
            if let Some(img) = image {
                *img = copy_asset(&img, &new_prefab.name);
            }
        }
        Workflow::TextToModel { image, model, .. } => {
            if let Some(img) = image {
                *img = copy_asset(&img, &new_prefab.name);
            }
            if let Some(m) = model {
                *m = copy_asset(&m, &new_prefab.name);
            }
        }
    }

    commands.spawn((
        Transform::from_translation(trans.translation + Vec3::new(2.0, 0., 0.1)), // offset z so no z fighting
        Name::new(new_name.clone()),
        new_prefab,
    ));
}

/// can be used to copy or rename the image, delete meta file
fn copy_asset(img: &String, name: &String) -> String {
    let asset_path = Path::new("./assets/");

    let image_path = Path::new(&img);
    let file_ext = image_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    let new_image_path = Path::new("ref").join(format!("{}.{}", name, file_ext));

    let src = asset_path.join(&image_path);
    let dst = asset_path.join(&new_image_path);

    dbg!("Copying asset from {:?} to {:?}", &src, &dst);
    std::fs::copy(&src, &dst).unwrap_or_default();

    let path = new_image_path.to_str().unwrap().to_string();
    path
}

fn rename_asset(img: &String, name: &String) -> String {
    let asset_path = Path::new("./assets/");

    let image_path = Path::new(&img);
    let file_ext = image_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("png");
    let new_image_path = Path::new("ref").join(format!("{}.{}", name, file_ext));

    let src = asset_path.join(&image_path);
    let dst = asset_path.join(&new_image_path);

    dbg!("Renaming asset from {:?} to {:?}", &src, &dst);
    std::fs::rename(&src, &dst).unwrap_or_default();

    let path = new_image_path.to_str().unwrap().to_string();
    path
}

fn remove_numeric_suffix(name: String) -> String {
    if let Some(pos) = name.rfind('_') {
        // Check if the characters after the underscore are all digits.
        if name[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
            return name[..pos].to_string();
        }
    }
    name.to_string()
}

#[derive(Event)]
pub struct Delete;

fn on_delete(trigger: Trigger<Delete>, mut commands: Commands, query: Query<&Prefab>) {
    let entity = trigger.target();

    let prefab = query.get(entity).unwrap();

    match &prefab.workflow {
        Workflow::StaticImage { image } => {
            if let Some(img) = image {
                std::fs::remove_file(Path::new("assets").join(img)).unwrap_or_default();
            }
        }
        Workflow::TextToImage { image, .. } => {
            if let Some(img) = image {
                std::fs::remove_file(Path::new("assets").join(img)).unwrap_or_default();
            }
        }
        Workflow::TextToModel { image, model, .. } => {
            if let Some(img) = image {
                std::fs::remove_file(Path::new("assets").join(img)).unwrap_or_default();
            }
            if let Some(m) = model {
                std::fs::remove_file(Path::new("assets").join(m)).unwrap_or_default();
            }
        }
    }

    commands.entity(entity).despawn();
}

#[derive(Event)]
pub struct Rename(pub String);

pub fn on_rename(trigger: Trigger<Rename>, mut query: Query<&mut Prefab>) {
    let entity = trigger.target();
    let mut new_name = trigger.0.clone();
    let names = query.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
    new_name = create_unique_name(&new_name, names);

    let mut prefab = query.get_mut(entity).unwrap();
    prefab.name = new_name.clone();

    match &mut prefab.workflow {
        Workflow::TextToImage { image, .. } => {
            if let Some(img) = image {
                *img = rename_asset(&img, &new_name);
            }
        }
        Workflow::TextToModel { image, model, .. } => {
            if let Some(img) = image {
                *img = rename_asset(&img, &new_name);
            }
            if let Some(m) = model {
                *m = rename_asset(&m, &new_name);
            }
        }
        Workflow::StaticImage { image } => {
            if let Some(img) = image {
                *img = rename_asset(&img, &new_name);
            }
        }
    }
}

fn create_unique_name(new_name: &String, names: Vec<String>) -> String {
    let mut new_name = new_name.clone();
    if names.contains(&new_name) {
        // Rename
        let short_name = remove_numeric_suffix(new_name.clone());
        let mut name = short_name.clone();
        let mut i = 1;
        while names.contains(&name) {
            i += 1;
            name = format!("{}_{}", short_name, i);
        }
        new_name = name.clone();
    }
    new_name
}

// each prefab workflow can be made up of multiple stages, if none are selected, runs all stages
#[derive(Event)]
pub struct Generate(pub Option<u8>);

// TODO: setup history tracking, or progress from api
const IMAGE_TIME: f32 = 10.0;
const MODEL_TIME: f32 = 90.0;

pub fn on_generate(
    trigger: Trigger<Generate>,
    mut query: Query<&mut Prefab>,
    runtime: ResMut<TokioTasksRuntime>,
    mut rng: GlobalEntropy<WyRand>,
    mut commands: Commands,
) {
    let e = trigger.target();
    let mut prefab = query.get_mut(e).unwrap();
    let stage = trigger.0;
    let name = prefab.name.clone();
    match &mut prefab.workflow {
        Workflow::StaticImage { .. } => {}
        Workflow::TextToImage {
            image,
            seed,
            seed_random,
            prompt,
        } => {
            commands.entity(e).insert(WorkflowProgress {
                timer: Timer::new(Duration::from_secs_f32(IMAGE_TIME), TimerMode::Once),
            });

            let start = Instant::now();
            // only 1 stage here
            let image_path = get_image_path(&name, image);
            let new_seed = update_seed(&mut rng, seed, seed_random);
            let prompt = prompt.clone();
            runtime.spawn_background_task(async move |mut ctx| {
                generate_image(&name, &image_path, new_seed, &prompt)
                    .await
                    .unwrap();

                tokio::time::sleep(Duration::from_secs_f32(FILE_DELAY)).await;

                ctx.run_on_main_thread(move |ctx| {
                    ctx.world
                        .trigger_targets(RefreshImage(image_path.clone()), e);
                    ctx.world.entity_mut(e).remove::<WorkflowProgress>();
                    let end = Instant::now();
                    info!("TextToImage generated in {:?}", end.duration_since(start));
                })
                .await;
            });
        }
        Workflow::TextToModel {
            image,
            seed,
            seed_random,
            prompt,
            model,
            num_faces,
        } => {
            commands.entity(e).insert(WorkflowProgress {
                timer: Timer::new(
                    Duration::from_secs_f32(match stage {
                        Some(x) => match x {
                            0 => IMAGE_TIME,
                            1 => MODEL_TIME,
                            _ => IMAGE_TIME + MODEL_TIME,
                        },
                        None => IMAGE_TIME + MODEL_TIME,
                    }),
                    TimerMode::Once,
                ),
            });
            let start = Instant::now();

            let image_path = get_image_path(&name, image);
            let model_path = get_model_path(&name, model);
            let new_seed = update_seed(&mut rng, seed, seed_random);
            let prompt = prompt.clone();
            let num_faces = *num_faces;
            runtime.spawn_background_task(async move |mut ctx: TaskContext| {
                // if stage is None, or stage == Some(0) run image
                if stage.is_none() || stage == Some(0) {
                    generate_image(&name, &image_path, new_seed, &prompt)
                        .await
                        .unwrap();

                    tokio::time::sleep(Duration::from_secs_f32(FILE_DELAY)).await;

                    let image_path_c = image_path.clone();
                    ctx.run_on_main_thread(move |ctx| {
                        ctx.world
                            .trigger_targets(RefreshImage(image_path_c.clone()), e);

                        let stage0 = Instant::now();
                        info!("TextToImage stage 0 in {:?}", stage0.duration_since(start));
                    })
                    .await;
                }

                if stage.is_none() || stage == Some(1) {
                    generate_model(&name, &image_path, &model_path, new_seed, num_faces)
                        .await
                        .unwrap();

                    tokio::time::sleep(Duration::from_secs_f32(FILE_DELAY)).await;

                    ctx.run_on_main_thread(move |ctx| {
                        ctx.world
                            .trigger_targets(RefreshModel(model_path.clone()), e);
                        let end = Instant::now();
                        info!("TextToImage generated in {:?}", end.duration_since(start));
                    })
                    .await;
                }
                ctx.run_on_main_thread(move |ctx| {
                    ctx.world.entity_mut(e).remove::<WorkflowProgress>();
                })
                .await;
            });
        }
    }
}

async fn generate_image(
    name: &String,
    image_path: &String,
    new_seed: u32,
    prompt: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the JSON workflow.
    let mut workflow: Value = serde_json::from_str(include_str!("workflows/ref_image_gen.json"))?;

    // update the seed
    if let Some(text_value) = workflow.pointer_mut("/9/inputs/seed") {
        *text_value = json!(new_seed);
    }
    // update the prompt text
    if let Some(text_value) = workflow.pointer_mut("/11/inputs/text") {
        *text_value = json!(prompt);
    }
    // update save_path
    if let Some(text_value) = workflow.pointer_mut("/13/inputs/value") {
        *text_value = json!(name);
    }

    // Connect to the websocket.
    let (client, client_id, mut ws) = comfy::connect_comfy().await.unwrap();

    // Wait for execution to complete and download the images.
    let images = comfy::get_images(&mut ws, &client, &workflow, &client_id).await?;
    assert!(images.len() == 1, "Wrong number of images generated");

    ws.close(None).await?;

    for (_node_id, images_vec) in images.iter() {
        for (_i, image_data) in images_vec.iter().enumerate() {
            let file_path = Path::new("assets").join(&image_path);
            tokio::fs::write(&file_path, image_data).await?;
        }
    }
    Ok(())
}

async fn generate_model(
    name: &String,
    image_path: &String,
    model_path: &String,
    new_seed: u32,
    num_faces: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the JSON workflow.
    let mut workflow: Value = serde_json::from_str(include_str!("workflows/ref_3d_gen.json"))?;

    // update the seed gen mesh
    if let Some(text_value) = workflow.pointer_mut("/141/inputs/seed") {
        *text_value = json!(new_seed);
    }

    // update input image with what we call it when we upload it
    let filename = Path::new(&image_path)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    if let Some(text_value) = workflow.pointer_mut("/174/inputs/image") {
        *text_value = json!(&filename);
    }

    // update save_path
    if let Some(text_value) = workflow.pointer_mut("/175/inputs/value") {
        *text_value = json!(name);
    }

    if let Some(text_value) = workflow.pointer_mut("/59/inputs/max_facenum") {
        *text_value = json!(num_faces);
    }

    // Connect to the websocket.
    let (client, client_id, mut ws) = comfy::connect_comfy().await.unwrap();

    // upload image
    let file_path = Path::new("assets")
        .join(&image_path)
        .to_string_lossy()
        .to_string();
    comfy::upload_image(&client, file_path, filename.clone())
        .await
        .unwrap();
    // Wait for execution to complete and download the images.
    let models = comfy::get_models(&mut ws, &client, &workflow, &client_id, "154").await?;

    ws.close(None).await?;
    //dbg!("models", &models);

    for (_node_id, images_vec) in models.iter() {
        for (i, image_data) in images_vec.iter().enumerate() {
            let file_path = Path::new("assets").join(&model_path);
            tokio::fs::write(&file_path, image_data).await?;
            info!("Saved model to {:?}", file_path);
            if i > 0 {
                warn!("more than one model generated, only keeping first");
                break;
            } 
        }
    }
    Ok(())
}

fn get_image_path(name: &String, image: &Option<String>) -> String {
    let path = match image {
        Some(img) => img.clone(),
        None => Path::new("ref")
            .join(format!("{}.png", name))
            .to_str()
            .unwrap()
            .to_string(),
    };
    path
}

fn get_model_path(name: &String, model: &Option<String>) -> String {
    let path = match model {
        Some(img) => img.clone(),
        None => Path::new("ref")
            .join(format!("{}.glb", name))
            .to_str()
            .unwrap()
            .to_string(),
    };
    path
}

// creates new seed and sets it if needed, returns the new seed
fn update_seed(
    rng: &mut bevy_rand::prelude::Entropy<WyRand>,
    seed: &mut u32,
    seed_random: &mut bool,
) -> u32 {
    let new_seed = if *seed_random {
        let x = rng.r#gen::<u32>();
        *seed = x;
        x
    } else {
        *seed
    };
    new_seed
}

// called when there is a new Image available
#[derive(Event)]
pub struct RefreshImage(pub String);

pub fn on_refresh_image(
    trigger: Trigger<RefreshImage>,
    mut query: Query<(&mut Prefab, &MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let e = trigger.target();
    let (mut p, mat_handle) = query.get_mut(e).unwrap();
    // update the image in the prefab
    //let mut changed = false;
    match &mut p.workflow {
        Workflow::TextToModel { image, .. } => {
            if *image != Some(trigger.0.clone()) {
                //changed = true;
                *image = Some(trigger.0.clone());
            }
        }
        Workflow::TextToImage { image, .. } => {
            if *image != Some(trigger.0.clone()) {
                //changed = true;
                *image = Some(trigger.0.clone());
            }
        }
        _ => unreachable!(),
    }

    // TODO: shouldnt need to do this, but not reloading not working without
    // update the material
    //if changed {
    //     warn!("Updating image {:?}", &trigger.0);
    let mat = materials.get_mut(&mat_handle.0).unwrap();
    mat.base_color_texture = Some(asset_server.load(&trigger.0));
    //}
}

#[derive(Event)]
pub struct RefreshModel(pub String);

pub fn on_refresh_model(
    trigger: Trigger<RefreshModel>,
    mut query: Query<(&mut Prefab, &Children)>,
    //mut scene_query: Query<&SceneRoot>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let e = trigger.target();
    let (mut p, children) = query.get_mut(e).unwrap();
    //let mut changed = false;
    // update path
    match &mut p.workflow {
        Workflow::TextToModel { model, .. } => {
            if *model != Some(trigger.0.clone()) {
                //changed = true;
                *model = Some(trigger.0.clone());
            }
        }
        _ => unreachable!(),
    }

    // update the model
    // TODO: shouldnt need to do this
    //if changed {
    let child = children[0];
    commands.entity(child).insert(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(trigger.0.clone())),
    ));
    //}
}
