use bevy::{
    color::palettes::tailwind,
    ecs::{component::HookContext, system::SystemState, world::DeferredWorld},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_inspector_egui::{
    bevy_egui::EguiContext,
    egui::{self},
};
use strum::IntoEnumIterator;

use crate::{Generate, Prefab, Rename, Save, Selected, SpawnPrefab, Workflow};

const NORMAL_BUTTON: Color = Color::Srgba(tailwind::SLATE_500);
const NORMAL_BUTTON_BORDER: Color = Color::Srgba(tailwind::SLATE_600);
//const NORMAL_BUTTON_TEXT: Color = Color::Srgba(tailwind::SLATE_100);
const HOVERED_BUTTON: Color = Color::Srgba(tailwind::SLATE_600);
const HOVERED_BUTTON_BORDER: Color = Color::Srgba(tailwind::SLATE_700);
const PRESSED_BUTTON: Color = Color::Srgba(tailwind::SLATE_700);
const PRESSED_BUTTON_BORDER: Color = Color::Srgba(tailwind::SLATE_800);
const PANEL_BACKGROUND: Color = Color::Srgba(tailwind::GRAY_900);
const PANEL_BORDER: Color = Color::Srgba(tailwind::GRAY_800);

#[derive(Component)]
#[require(
    Button,
    Node = Node {
        padding: UiRect::all(Val::Px(2.0)),
        margin: UiRect::all(Val::Px(4.0)),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    },
    BackgroundColor(NORMAL_BUTTON),
    //BorderColor(PANEL_BORDER),
    Outline = Outline {
        width: Val::Px(2.),
        color: NORMAL_BUTTON_BORDER,
        ..default()
    },
    BorderRadius::all(Val::Px(5.)),
)]
#[component(on_add = on_add_quick_button)]
pub struct QuickButton;

pub fn on_add_quick_button(mut world: DeferredWorld<'_>, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .entity(entity)
        .observe(update_colors_on::<Pointer<Over>>(
            HOVERED_BUTTON,
            HOVERED_BUTTON_BORDER,
        ))
        .observe(update_colors_on::<Pointer<Out>>(
            NORMAL_BUTTON,
            NORMAL_BUTTON_BORDER,
        ))
        .observe(update_colors_on::<Pointer<Pressed>>(
            PRESSED_BUTTON,
            PRESSED_BUTTON_BORDER,
        ))
        .observe(update_colors_on::<Pointer<Released>>(
            HOVERED_BUTTON,
            HOVERED_BUTTON_BORDER,
        ));
    //  .insert(
    //      BackgroundColor(normal_button),
    //  )
}

fn update_colors_on<E>(
    background: Color,
    outline: Color,
) -> impl Fn(Trigger<E>, Query<(&mut BackgroundColor, &mut Outline)>) {
    // An observer closure that captures `new_material`. We do this to avoid needing to write four
    // versions of this observer, each triggered by a different event and with a different hardcoded
    // material. Instead, the event type is a generic, and the material is passed in.
    move |trigger, mut query| {
        if let Ok((mut bg, mut out)) = query.get_mut(trigger.target()) {
            bg.0 = background;
            out.color = outline;
        }
    }
}

#[derive(Component)]
#[require(
    Node = Node {
        width: Val::Px(30.0),
        height: Val::Px(30.0),
        ..default()
    },
)]
pub struct QuickButtonInner;

pub fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Name::new("Quick Panel"),
            Node {
                position_type: PositionType::Absolute,

                right: Val::Px(10.),
                bottom: Val::Px(10.),
                padding: UiRect::all(Val::Px(4.0)),
                // horizontally center child text
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..Default::default()
            },
            BorderRadius::all(Val::Px(5.)),
            BackgroundColor(PANEL_BACKGROUND),
            BorderColor(PANEL_BORDER),
            Outline {
                width: Val::Px(2.),
                color: Color::WHITE,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Quick Add Button"),
                    QuickButton,
                    children!((
                        QuickButtonInner,
                        ImageNode::new(asset_server.load("textures/icon/white/plus.png")),
                    )),
                ))
                .observe(
                    |_trigger: Trigger<Pointer<Click>>, mut commands: Commands| {
                        commands.send_event(SpawnPrefab);
                    },
                );
            parent
                .spawn((
                    Name::new("Quick Save Button"),
                    QuickButton,
                    children!((
                        QuickButtonInner,
                        ImageNode::new(asset_server.load("textures/icon/white/checkmark.png")),
                    )),
                ))
                .observe(
                    |_trigger: Trigger<Pointer<Click>>, mut commands: Commands| {
                        commands.send_event(Save);
                    },
                );

            parent
                .spawn((
                    Name::new("Quick Exit Button"),
                    QuickButton,
                    children!((
                        QuickButtonInner,
                        ImageNode::new(asset_server.load("textures/icon/white/exitRight.png")),
                    )),
                ))
                .observe(
                    |_trigger: Trigger<Pointer<Click>>, mut commands: Commands| {
                        commands.send_event(AppExit::Success);
                    },
                );
        });
}

pub fn ui_select(world: &mut World) {
    let mut egui_context = world
        .query_filtered::<&mut EguiContext, With<PrimaryWindow>>()
        .single(world)
        .expect("No EguiContext found")
        .clone();

    let mut system_state: SystemState<(Commands, Query<(Entity, &mut Prefab), With<Selected>>)> =
        SystemState::new(world);

    let (mut cmd, mut query) = system_state.get_mut(world);

    egui::Window::new("Select").show(egui_context.get_mut(), |ui| {
        egui::ScrollArea::both().show(ui, |ui| {
            // let changed = ui.text_edit_singleline(&mut prefab.name);
            // if changed.changed() {
            //     world
            //         .query_filtered::<&mut Prefab, With<Selected>>()
            //         .single(world)
            //         .name = prefab.name.clone();
            // }
            //bevy_inspector_egui::bevy_inspector::ui_for_entities_filtered(world, ui, false, &Filter::<With<Prefab>>::all());
            for (e, mut p) in query.iter_mut() {
                let id = egui::Id::new("prefab ui").with(e);
                let mut changed = false;
                egui::Grid::new(id)
                    .num_columns(2)
                    .spacing([16.0, 4.0]) // [horizontal, vertical] spacing
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Name");
                        let mut new_name = p.name.clone();
                        changed |= ui.add(egui::TextEdit::singleline(&mut new_name)).changed();
                        if changed {
                            cmd.trigger_targets(Rename(new_name), e);
                        }
                        ui.end_row();

                        ui.label("Workflow");
                        let mut workflow_copy = p.workflow.clone();
                        let mut workflow_changed = false;
                        egui::ComboBox::from_id_salt(id)
                            .selected_text(format!("{}", workflow_copy))
                            .show_ui(ui, |ui| {
                                for variant in Workflow::iter() {
                                    workflow_changed |= ui
                                        .selectable_value(
                                            &mut workflow_copy,
                                            variant.clone(),
                                            format!("{}", &variant),
                                        )
                                        .changed();
                                }
                            });

                        // handles keeping useful data when changing workflow types
                        if workflow_changed {
                            changed = true;
                            match (workflow_copy, &p.workflow) {
                                (
                                    Workflow::StaticImage { .. },
                                    Workflow::TextToImage { image, .. },
                                ) => {
                                    p.workflow = Workflow::StaticImage {
                                        image: image.clone(),
                                    };
                                }
                                (Workflow::StaticImage { .. }, Workflow::StaticImage { .. }) => {
                                    unreachable!()
                                }
                                (
                                    Workflow::StaticImage { .. },
                                    Workflow::TextToModel { image, .. },
                                ) => {
                                    p.workflow = Workflow::StaticImage {
                                        image: image.clone(),
                                    };
                                }
                                (Workflow::TextToImage { .. }, Workflow::StaticImage { image }) => {
                                    p.workflow = Workflow::TextToImage {
                                        seed: 0,
                                        seed_random: false,
                                        prompt: "".to_string(),
                                        image: image.clone(),
                                    };
                                }
                                (
                                    Workflow::TextToImage { .. },
                                    Workflow::TextToModel {
                                        image,
                                        seed,
                                        seed_random,
                                        prompt,
                                        ..
                                    },
                                ) => {
                                    p.workflow = Workflow::TextToImage {
                                        image: image.clone(),
                                        seed: *seed,
                                        seed_random: *seed_random,
                                        prompt: prompt.clone(),
                                    };
                                }
                                (Workflow::TextToImage { .. }, Workflow::TextToImage { .. }) => {
                                    unreachable!()
                                }
                                (Workflow::TextToModel { .. }, Workflow::StaticImage { image }) => {
                                    p.workflow = Workflow::TextToModel {
                                        seed: 0,
                                        seed_random: false,
                                        prompt: "".to_string(),
                                        image: image.clone(),
                                        num_faces: 50000,
                                        model: None,
                                    };
                                }
                                (
                                    Workflow::TextToModel { .. },
                                    Workflow::TextToImage {
                                        image,
                                        seed,
                                        seed_random,
                                        prompt,
                                    },
                                ) => {
                                    p.workflow = Workflow::TextToModel {
                                        seed: *seed,
                                        seed_random: *seed_random,
                                        num_faces: 50000,
                                        prompt: prompt.clone(),
                                        image: image.clone(),
                                        model: None,
                                    };
                                }

                                (Workflow::TextToModel { .. }, Workflow::TextToModel { .. }) => {
                                    unreachable!()
                                }
                            }
                        }
                        ui.end_row();

                        let mut enable_generate = true;
                        match &mut p.workflow {
                            Workflow::StaticImage { image } => {
                                image_widget(ui, image);
                                enable_generate = false;
                            }
                            Workflow::TextToImage {
                                image,
                                prompt,
                                seed,
                                seed_random,
                            } => {
                                changed |= prompt_widget(ui, prompt);
                                image_widget(ui, image);
                                seed_wigit(ui, seed, seed_random);
                            }
                            Workflow::TextToModel {
                                prompt,
                                image,
                                model,
                                seed,
                                seed_random,
                                num_faces,
                            } => {
                                changed |= prompt_widget(ui, prompt);
                                image_widget(ui, image);
                                model_widget(ui, model);
                                changed |= seed_wigit(ui, seed, seed_random);

                                ui.label("Faces");
                                changed |= ui
                                    .add(
                                        egui::Slider::new(num_faces, 0..=u32::MAX)
                                            .text("Faces")
                                            .step_by(1000.)
                                            .clamping(egui::SliderClamping::Always),
                                    )
                                    .changed();
                                ui.end_row();

                                ui.label("");
                                if ui
                                    .add_enabled(
                                        enable_generate,
                                        egui::Button::new("Generate Image")
                                            .min_size(egui::Vec2::new(ui.available_width(), 30.0)),
                                    )
                                    .clicked()
                                {
                                    cmd.trigger_targets(Generate(Some(0)), e);
                                }
                                ui.end_row();

                                ui.label("");
                                if ui
                                    .add_enabled(
                                        enable_generate,
                                        egui::Button::new("Generate Model")
                                            .min_size(egui::Vec2::new(ui.available_width(), 30.0)),
                                    )
                                    .clicked()
                                {
                                    cmd.trigger_targets(Generate(Some(1)), e);
                                }
                                ui.end_row();
                            }
                        }

                        ui.label("");
                        if ui
                            .add_enabled(
                                enable_generate,
                                egui::Button::new("Generate Full")
                                    .min_size(egui::Vec2::new(ui.available_width(), 30.0)),
                            )
                            .clicked()
                        {
                            cmd.trigger_targets(Generate(None), e);
                        }
                        ui.end_row();
                    });
            }
            //ui_for_entities_filtered(world, ui, &Filter::<(With<Prefab>, With<Selected>)>::all());

            //bevy_inspector_egui::bevy_inspector::ui_for_value(prefab.deref_mut(), ui, world);

            ui.allocate_space(ui.available_size());
        });
    });

    system_state.apply(world);
}

fn image_widget(ui: &mut egui::Ui, p: &mut Option<String>) {
    ui.label("Image");
    if let Some(text) = p {
        ui.add_enabled(
            false,
            egui::TextEdit::singleline(text).desired_width(f32::INFINITY), // optional, makes it fill width
        );
    } else {
        ui.label("None");
    }
    ui.end_row();
}

fn model_widget(ui: &mut egui::Ui, p: &mut Option<String>) {
    ui.label("Model");
    if let Some(text) = p {
        ui.add_enabled(
            false,
            egui::TextEdit::singleline(text).desired_width(f32::INFINITY), // optional, makes it fill width
        );
    } else {
        ui.label("None");
    }
    ui.end_row();
}

fn prompt_widget(ui: &mut egui::Ui, prompt: &mut String) -> bool {
    let mut changed = false;
    ui.label("Prompt");
    changed |= ui.text_edit_multiline(prompt).changed();
    ui.end_row();
    changed
}

fn seed_wigit(ui: &mut egui::Ui, seed: &mut u32, random: &mut bool) -> bool {
    let mut changed = false;

    ui.label("Seed");
    changed |= ui
        .add(
            egui::Slider::new(seed, 0..=u32::MAX)
                .text("Seed")
                .clamping(egui::SliderClamping::Always),
        )
        .changed();
    ui.end_row();

    ui.label("Seed");
    if ui.checkbox(random, "Randomize").changed() {
        changed = true;
    }
    ui.end_row();

    changed
}
