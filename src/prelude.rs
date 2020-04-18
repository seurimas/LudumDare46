pub use amethyst::{core::Named, ecs::*, prelude::*};

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
