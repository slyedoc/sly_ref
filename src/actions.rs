use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::{paste, Delete, Duplicate, Prefab, Save, Selected, SpawnPrefab};

pub struct AppActionPlugin;

pub(super) const DEFAULT_SPEED: f32 = 0.1;
pub(super) const DEFAULT_ROTATION: f32 = 0.003;

impl Plugin for AppActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_actions_marker::<AppActions>()
            .add_observer(binding)
            .add_observer(duplicate_selected)
            .add_observer(delete_selected)
            .add_observer(save)
            .add_observer(exit)
            .add_observer(paste)
            .add_observer(spawn)
            //camera
            .add_observer(apply_movement)
            .add_observer(apply_assend)
            .add_observer(apply_rotate)
            .add_systems(PreStartup, |mut commands: Commands| {
                commands.spawn(Actions::<AppActions>::default());
            });
    }
}

fn binding(trigger: Trigger<Binding<AppActions>>, mut players: Query<&mut Actions<AppActions>>) {
    let mut actions = players.get_mut(trigger.target()).unwrap();

    // Spawn
    actions
        .bind::<SpawnAction>()
        .to(KeyCode::KeyA.with_mod_keys(ModKeys::CONTROL))
        .with_conditions(JustPress::default());

    // Duplicate
    actions
        .bind::<DuplicateAction>()
        .to(KeyCode::KeyD.with_mod_keys(ModKeys::CONTROL))
        .with_conditions(JustPress::default());



    // Save
    actions
        .bind::<SaveAction>()
        .to(KeyCode::KeyS.with_mod_keys(ModKeys::CONTROL))
        .with_conditions(JustPress::default());

    // Delete
    actions
        .bind::<DeleteAction>()
        .to(KeyCode::Delete)
        .with_conditions(JustPress::default());

    // Exit
    actions
        .bind::<ExitAction>()
        .to(KeyCode::Escape)
        .with_conditions((JustPress::default(),));

    // Paste
    actions
        .bind::<PasteAction>()
        .to(KeyCode::KeyV.with_mod_keys(ModKeys::CONTROL))
        .with_conditions(JustPress::default());

    // Movement
    actions.bind::<EnableSprint>().to(KeyCode::ShiftLeft);

    actions
        .bind::<Move>()
        .to((
            Cardinal::wasd_keys()
                .with_conditions_each(BlockBy::<EnableSprint>::default()),            
            Cardinal::wasd_keys()
                .with_conditions_each(Chord::<EnableSprint>::default())
                .with_modifiers_each(Scale::splat(10.0)),
            GamepadStick::Left,
            Cardinal::arrow_keys()
        ))
        // Don't trigger the action when the chord is active.
        .with_modifiers((
            //DeadZone::default(), // Apply non-uniform normalization to ensure consistent speed, otherwise diagonal movement will be faster.
            SmoothNudge::default(), // Make movement smooth and independent of the framerate. To only make it framerate-independent, use `DeltaScale`.
            Scale::splat(DEFAULT_SPEED), // Additionally multiply by a constant to achieve the desired speed.
        ));

    actions.bind::<Assend>()
        .to(Bidirectional {
            positive: KeyCode::KeyQ,
            negative: KeyCode::KeyE,
        })
        .with_modifiers((
            //DeadZone::default(), // Apply non-uniform normalization to ensure consistent speed, otherwise diagonal movement will be faster.
            SmoothNudge::default(), // Make movement smooth and independent of the framerate. To only make it framerate-independent, use `DeltaScale`.
            Scale::splat(DEFAULT_SPEED), // Additionally multiply by a constant to achieve the desired speed.
        ));

    actions.bind::<EnableLook>().to(MouseButton::Right);

    actions
        .bind::<Look>()
        .to((Input::mouse_motion().with_conditions(Chord::<EnableLook>::default()),
         GamepadStick::Right))
        //.with_conditions((Chord::<EnableLook>::default(),))
        .with_modifiers((
            //DeadZone::default(), // Apply non-uniform normalization to ensure consistent speed, otherwise diagonal movement will be faster.
            SmoothNudge::default(), // Make movement smooth and independent of the framerate. To only make it framerate-independent, use `DeltaScale`.
            Scale::splat(DEFAULT_ROTATION), // Additionally multiply by a constant to achieve the desired speed.
        ));
}

fn apply_movement(
    trigger: Trigger<Fired<Move>>,
    mut camera_transform: Single<&mut Transform, With<Camera>>,
) {
    let change = vec3(trigger.value.x, 0.0, -trigger.value.y);
    let rot = camera_transform.rotation;
    camera_transform.translation += rot * change;
}

fn apply_assend(
    trigger: Trigger<Fired<Assend>>,
    mut camera_transform: Single<&mut Transform, With<Camera>>,
) {
    let change = vec3(0.0, trigger.value, 0.0);
    let rot = camera_transform.rotation;
    camera_transform.translation += rot * change;
}

fn apply_rotate(
    trigger: Trigger<Fired<Look>>,
    mut camera_transform: Single<&mut Transform, With<Camera>>,
) {
    let change = trigger.value;

    let (mut yaw, mut pitch, _roll) = camera_transform.rotation.to_euler(EulerRot::YXZ);
    yaw -= change.x;
    pitch -= change.y;

    pitch = pitch.clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

    camera_transform.rotation =
        Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
}

#[derive(ActionsMarker)]
#[actions_marker(priority = 1)]
pub struct AppActions;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct DuplicateAction;

fn duplicate_selected(
    _trigger: Trigger<Fired<DuplicateAction>>,
    selected: Query<Entity, (With<Selected>, With<Prefab>)>,
    mut commands: Commands,
) {
    info!("Duplicating selected");
    for e in selected.iter() {
        commands.trigger_targets(Duplicate, e);
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct DeleteAction;

fn delete_selected(
    _trigger: Trigger<Fired<DeleteAction>>,
    selected: Query<Entity, (With<Selected>, With<Prefab>)>,
    mut commands: Commands,
) {
    info!("delete selected");
    for e in selected.iter() {
        commands.trigger_targets(Delete, e);
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct SpawnAction;

fn spawn(_trigger: Trigger<Fired<SaveAction>>, mut commands: Commands) {
    commands.send_event(SpawnPrefab);
}



#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct SaveAction;

fn save(_trigger: Trigger<Fired<SaveAction>>, mut commands: Commands) {
    info!("save");
    commands.send_event(Save);
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct ExitAction;

fn exit(_trigger: Trigger<Fired<ExitAction>>, mut commands: Commands) {
    commands.send_event(AppExit::Success);
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct PasteAction;

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
struct Move;


#[derive(Debug, InputAction)]
#[input_action(output = f32)]
struct Assend;


#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
struct Look;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct EnableLook;


#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct EnableSprint;