use bevy::prelude::*;
use bevy_health_bar3d::prelude::*;

#[derive(Component, Reflect)]
#[reflect()]
pub struct WorkflowProgress {
    pub timer: Timer,
}

impl Percentage for WorkflowProgress {
    fn value(&self) -> f32 {
        self.timer.fraction()
    }
}

pub fn update_progress(time: Res<Time>, mut query: Query<(Entity, &mut WorkflowProgress)>) {
    for (_e, mut progress) in query.iter_mut() {
        progress.timer.tick(time.delta());
    }
}
