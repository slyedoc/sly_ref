use bevy::prelude::*;
use bevy_enhanced_input::events::Fired;
//use billboard::prelude::*;
use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType, Seat};

use crate::{PasteAction, Prefab, Workflow};

pub fn paste( 
    _trigger: Trigger<Fired<PasteAction>>,
    mut commands: Commands,
    camera_transform: Single<&Transform, With<Camera>>,  
) {
    info!("Paste event triggered");
    use std::io::Read;

    let result = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Any);
    match result {
        Ok((mut pipe, _)) => {
            let mut contents = vec![];
            if let Ok(_) = pipe.read_to_end(&mut contents) {
                let clipboard = String::from_utf8_lossy(&contents).to_string();
                info!("Clipboard contents: {:?}", &clipboard);

                if clipboard.ends_with(".png") {
                    let path = std::path::Path::new(&clipboard);
                    let file_name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("clipboard_image.png")
                        .to_string();
                    let file_stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("clipboard_image")
                        .to_string();

                    let new_path = std::path::Path::new("assets/ref/").join(&file_name);

                    let new_asset_path = format!("ref/{}", &file_name);
                    // copy to ref assets
                    std::fs::copy(&path, &new_path).expect("Failed to copy file");

                    let pos = camera_transform.translation + camera_transform.forward() * 4.0;
                    commands.spawn((
                        Transform::from_translation(pos),
                        Name::new(file_stem.clone()),
                        Prefab {
                            name: file_stem,
                            workflow: Workflow::StaticImage {
                                image: Some(new_asset_path),
                            },
                        },
                    ));
                } else {
                    warn!("Clipboard contents not an image: {:?}", &clipboard);
                }
            }
        }
        Err(err) => {
            error!("Error pasting: {:?}", err);
        }
    }
}

// TODO: doesnt work on wayland
pub fn file_drop(mut evr_dnd: EventReader<FileDragAndDrop>) {
    for ev in evr_dnd.read() {
        // TODO: on wayland this event never fires
        dbg!("File drop event never fire!!!!!!");
        match ev {
            FileDragAndDrop::DroppedFile { window, path_buf } => {
                info!("Dropped file: {:?} at {:?}", path_buf, window);
                //let texture_handle = asset_server.load(path_buf.to_str().unwrap().to_string());

                // commands.spawn(
                //     SpriteBundle {
                //         texture: texture_handle,
                //         transform: Transform::from_xyz(world_cursor.0.x, world_cursor.0.y, 0.0),
                //         ..default()
                //     });
            }
            FileDragAndDrop::HoveredFile {
                window: _,
                path_buf: _,
            } => {
                // On wayland this sometimes prints multiple times for one drop
                info!("Hovered file");
            }
            FileDragAndDrop::HoveredFileCanceled { window: _ } => {
                info!("File canceled!");
            }
        }
    }
}
