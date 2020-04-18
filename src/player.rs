use crate::assets::{AnimationId, Direction, PrefabStorage};
use crate::physics::*;
use amethyst::{
    animation::*,
    assets::Handle,
    core::{bundle::SystemBundle, transform::*},
    ecs::*,
    error::Error,
    input::{InputHandler, StringBindings},
    prelude::*,
    renderer::{camera::*, SpriteRender},
};
use na::{Isometry2, Vector2};
use ncollide2d::shape::*;
use nphysics2d::object::*;

const ARENA_WIDTH: f32 = 480.0;
const ARENA_HEIGHT: f32 = 320.0;
fn standard_camera() -> Camera {
    Camera::standard_2d(ARENA_WIDTH, ARENA_HEIGHT)
}

pub fn initialize_camera(builder: impl Builder, player: Entity) -> Entity {
    // Setup camera in a way that our screen covers whole arena and (0, 0) is in the bottom left.
    let mut transform = Transform::default();
    transform.set_translation_xyz(0.0, 0.0, 1.0);

    builder
        .with(standard_camera())
        .with(Parent { entity: player })
        .with(transform)
        .build()
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Player {
    pub walk_speed: f32,
}

fn spawn_player(prefabs: &PrefabStorage, player_builder: impl Builder) -> Entity {
    let shape = ShapeHandle::new(Ball::new(16.0));
    let body = RigidBodyDesc::new().status(BodyStatus::Dynamic).mass(100.0);
    let collider = ColliderDesc::new(shape);
    player_builder
        .with(PhysicsDesc::new(body, collider))
        .with(prefabs.player.clone())
        .with(Transform::default())
        .with(Player { walk_speed: 100.0 })
        .build()
}

fn spawn_test_sensor(builder: impl Builder, player: Entity) -> Entity {
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(32.0, 32.0)));
    let collider = ColliderDesc::new(shape).sensor(true);
    builder
        .with(AttachedSensor::new(collider))
        .with(Parent { entity: player })
        .build()
}

pub fn spawn_player_world(world: &mut World) {
    let entities = world.entities();
    let update = world.write_resource::<LazyUpdate>();
    let builder = update.create_entity(&entities);
    let prefabs = world.read_resource::<PrefabStorage>();
    let player = spawn_player(&prefabs, builder);
    let builder = update.create_entity(&entities);
    initialize_camera(builder, player);
    let builder = update.create_entity(&entities);
    spawn_test_sensor(builder, player);
}

fn get_active_animation(
    control_set: &AnimationControlSet<AnimationId, SpriteRender>,
) -> Option<AnimationId> {
    for (id, animation) in control_set.animations.iter() {
        if animation.state.is_running() {
            println!("{:?}", id);
            return Some(*id);
        }
    }
    None
}

fn set_active_animation(
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

struct PlayerAnimationSystem;
impl<'s> System<'s> for PlayerAnimationSystem {
    type SystemData = (
        ReadStorage<'s, Player>,
        ReadStorage<'s, AnimationSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, SpriteRender>>,
        Entities<'s>,
    );

    fn run(&mut self, (player, animation_sets, mut control_sets, entities): Self::SystemData) {
        /*
        for (player, entity, animation_set) in (&player, &entities, &animation_sets).join() {
            // Creates a new AnimationControlSet for the entity
            let control_set = get_animation_set(&mut control_sets, entity).unwrap();
            if control_set.is_empty() {
                // Adds the `Fly` animation to AnimationControlSet and loops infinitely
                for direction in Direction::vec().iter() {
                    control_set.add_animation(
                        AnimationId::Idle(*direction),
                        &animation_set.get(&AnimationId::Idle(*direction)).unwrap(),
                        EndControl::Loop(None),
                        1.0,
                        AnimationCommand::Start,
                    );
                    control_set.add_animation(
                        AnimationId::Walk(*direction),
                        &animation_set.get(&AnimationId::Walk(*direction)).unwrap(),
                        EndControl::Loop(None),
                        1.0,
                        AnimationCommand::Start,
                    );
                }
            }
        }*/
    }
}

struct PlayerMovementSystem;
impl<'s> System<'s> for PlayerMovementSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, Physics<f32>>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, AnimationSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, SpriteRender>>,
        ReadStorage<'s, Player>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (input, mut physics, handles, animation_sets, mut control_sets, player, entities): Self::SystemData,
    ) {
        let x_tilt = input.axis_value("leftright");
        let y_tilt = input.axis_value("updown");
        if let (Some(x_tilt), Some(y_tilt)) = (x_tilt, y_tilt) {
            if let Some((entity, handle, player)) = (&entities, &handles, &player).join().next() {
                physics.set_velocity(
                    handle,
                    Vector2::new(x_tilt * player.walk_speed, y_tilt * player.walk_speed),
                );
                if let (Some(animation_set), Some(control_set)) = (
                    animation_sets.get(entity),
                    get_animation_set(&mut control_sets, entity),
                ) {
                    let direction = {
                        if x_tilt != 0.0 || y_tilt != 0.0 {
                            if f32::abs(x_tilt) >= f32::abs(y_tilt) {
                                if x_tilt >= 0.0 {
                                    Direction::East
                                } else {
                                    Direction::West
                                }
                            } else {
                                if y_tilt >= 0.0 {
                                    Direction::North
                                } else {
                                    Direction::South
                                }
                            }
                        } else if let Some(previous_animation) = get_active_animation(control_set) {
                            previous_animation.direction()
                        } else {
                            Direction::East
                        }
                    };
                    set_active_animation(
                        control_set,
                        if x_tilt != 0.0 || y_tilt != 0.0 {
                            AnimationId::Walk(direction)
                        } else {
                            AnimationId::Idle(direction)
                        },
                        &animation_set,
                        EndControl::Loop(None),
                        1.0,
                    );
                }
            }
        }
    }
}

pub struct PlayerBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for PlayerBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(PlayerAnimationSystem, "player_animation", &[]);
        dispatcher.add(PlayerMovementSystem, "player_movement", &[]);
        Ok(())
    }
}
