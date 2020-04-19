pub use crate::assets::{AnimationId, Direction};
pub use crate::physics::{AttachedSensor, PhysicsHandle};
pub use amethyst::{
    animation::*,
    core::{bundle::SystemBundle, transform::components::Parent, Named},
    ecs::*,
    error::Error,
    prelude::*,
    renderer::SpriteRender,
};
pub use na::Vector2;
pub use rand::prelude::*;

pub fn get_named_entity<'s>(
    entities: &Entities<'s>,
    names: &ReadStorage<'s, Named>,
    name: &'static str,
) -> Option<Entity> {
    for (entity, entity_name) in (entities, names).join() {
        if name.eq_ignore_ascii_case(&entity_name.name) {
            return Some(entity.clone());
        }
    }
    None
}

pub fn get_sensors<'s>(
    entities: &Entities<'s>,
    sensors: &ReadStorage<'s, AttachedSensor>,
    parents: &ReadStorage<'s, Parent>,
    parent_entity: Entity,
) -> Vec<(PhysicsHandle, Entity)> {
    let mut found = Vec::new();
    for (entity, sensor, parent) in (entities, sensors, parents).join() {
        if parent.entity == parent_entity {
            if let Some(handles) = sensor.handle {
                found.push((PhysicsHandle::new(handles.0, handles.1), entity));
            }
        }
    }
    found
}

pub fn get_active_animation(
    control_set: &AnimationControlSet<AnimationId, SpriteRender>,
) -> Option<AnimationId> {
    for (id, animation) in control_set.animations.iter() {
        if animation.state.is_running() {
            return Some(*id);
        }
    }
    None
}

/*
pub fn get_animation_progress(
    control_set: &AnimationControlSet<AnimationId, SpriteRender>,
) -> Option<f32> {
    for (id, animation) in control_set.animations.iter() {
        if animation.state.is_running() {
            println!("{:?}", animation.state);
            return Some(match animation.state {
                amethyst::animation::ControlState::Running(duration) => duration.as_secs_f32(),
                _ => 0.0,
            });
        }
    }
    None
}
*/

pub fn set_active_animation(
    control_set: &mut AnimationControlSet<AnimationId, SpriteRender>,
    id: AnimationId,
    animation_set: &AnimationSet<AnimationId, SpriteRender>,
    end: EndControl,
    rate_multiplier: f32,
) {
    let mut actives = Vec::new();
    for (active_id, animation) in control_set.animations.iter() {
        if animation.state.is_running() && *active_id != id {
            actives.push(*active_id);
        }
    }
    for active in actives {
        control_set.abort(active);
    }
    control_set.add_animation(
        id,
        &animation_set.get(&id).unwrap(),
        end,
        rate_multiplier,
        AnimationCommand::Start,
    );
}
