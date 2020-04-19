use crate::assets::{AnimationId, Direction, PrefabStorage};
use crate::combat::*;
use crate::physics::*;
use crate::prelude::*;
use amethyst::{
    animation::*,
    assets::Handle,
    core::{bundle::SystemBundle, transform::*},
    ecs::world::LazyBuilder,
    ecs::*,
    error::Error,
    input::{InputHandler, StringBindings},
    prelude::*,
    renderer::{camera::*, SpriteRender},
};
use na::{Isometry2, Vector2};
use ncollide2d::shape::*;
use nphysics2d::object::*;

const ARENA_WIDTH: f32 = 240.0;
const ARENA_HEIGHT: f32 = 160.0;
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

#[derive(Debug, PartialEq)]
pub enum PlayerState {
    Moving,
    Attacking(usize),
    Hit(f32),
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Player {
    pub walk_speed: f32,
    pub state: PlayerState,
    pub facing: Direction,
}

fn spawn_player(prefabs: &PrefabStorage, player_builder: LazyBuilder) -> Entity {
    let shape = ShapeHandle::new(Ball::new(8.0));
    let body = RigidBodyDesc::new()
        .status(BodyStatus::Dynamic)
        .mass(10.0)
        .linear_damping(0.0);
    let collider = ColliderDesc::new(shape);
    player_builder
        .with(PhysicsDesc::new(body, collider))
        .with(prefabs.player.clone())
        .with(Transform::default())
        .with(Player {
            walk_speed: 100.0,
            state: PlayerState::Moving,
            facing: Direction::South,
        })
        .with(Health {
            friendly: true,
            current_health: 100,
            last_attack: 0,
        })
        .named("player")
        .build()
}

const ATTACK_SENSOR_NAME: &'static str = "player_attack_sensor";

fn spawn_attack_sensor(builder: LazyBuilder, player: Entity, direction: Direction) -> Entity {
    let offset = direction.tilts() * 8.0 + direction.clockwise().tilts() * 4.0;
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(8.0, 8.0)));
    let collider = ColliderDesc::new(shape)
        .sensor(true)
        .position(Isometry2::new(offset, 0.0));
    builder
        .with(AttachedSensor::new(collider))
        .with(AttackHitbox {
            id: rand::random(),
            hit_type: HitType::FriendlyAttack,
        })
        .with(Parent { entity: player })
        .named(ATTACK_SENSOR_NAME)
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

struct PlayerAttackSystem;
impl<'s> System<'s> for PlayerAttackSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        Read<'s, Time>,
        Write<'s, Physics<f32>>,
        ReadStorage<'s, AttachedSensor>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Named>,
        ReadStorage<'s, AnimationSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, Player>,
        WriteStorage<'s, AttackHitbox>,
        Read<'s, LazyUpdate>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            input,
            time,
            mut physics,
            sensors,
            handles,
            names,
            animation_sets,
            mut control_sets,
            mut player,
            mut attacks,
            lazy,
            entities,
        ): Self::SystemData,
    ) {
        if let Some((entity, handle, player)) = (&entities, &handles, &mut player).join().next() {
            if let (Some(animation_set), Some(control_set)) = (
                animation_sets.get(entity),
                get_animation_set(&mut control_sets, entity),
            ) {
                match player.state {
                    PlayerState::Moving => {
                        if let Some(sensor) =
                            get_named_entity(&entities, &names, ATTACK_SENSOR_NAME)
                        {
                            println!("Deleting...");
                            lazy.exec(move |world| {
                                world.delete_entity(sensor);
                            });
                        }
                        if Some(true) == input.action_is_down("attack") {
                            player.state = PlayerState::Attacking(rand::random());
                            spawn_attack_sensor(
                                lazy.create_entity(&entities),
                                entity,
                                player.facing,
                            );
                            physics.set_velocity(handle, Vector2::new(0.0, 0.0));
                            set_active_animation(
                                control_set,
                                AnimationId::Attack(player.facing),
                                &animation_set,
                                EndControl::Stay,
                                1.0,
                            );
                        }
                    }
                    PlayerState::Attacking(attack_id) => {
                        if let Some(AnimationId::Attack(_)) = get_active_animation(control_set) {
                            if let Some(sensor) =
                                get_named_entity(&entities, &names, ATTACK_SENSOR_NAME)
                            {
                                attacks
                                    .insert(
                                        sensor,
                                        AttackHitbox {
                                            id: attack_id,
                                            hit_type: HitType::FriendlyAttack,
                                        },
                                    )
                                    .unwrap();
                            }
                        } else {
                            player.state = PlayerState::Moving;
                            set_active_animation(
                                control_set,
                                AnimationId::Idle(player.facing),
                                &animation_set,
                                EndControl::Loop(None),
                                1.0,
                            );
                        }
                    }
                    PlayerState::Hit(size) => {
                        if let Some(sensor) =
                            get_named_entity(&entities, &names, ATTACK_SENSOR_NAME)
                        {
                            lazy.exec(move |world| {
                                world.delete_entity(sensor);
                            });
                        }
                        if size < time.delta_seconds() {
                            player.state = PlayerState::Moving;
                        } else {
                            player.state = PlayerState::Hit(size - time.delta_seconds());
                            set_active_animation(
                                control_set,
                                AnimationId::Staggered(player.facing),
                                &animation_set,
                                EndControl::Stay,
                                1.0,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
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
        WriteStorage<'s, Player>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (input, mut physics, handles, animation_sets, mut control_sets, mut player, entities): Self::SystemData,
    ) {
        let x_tilt = input.axis_value("leftright");
        let y_tilt = input.axis_value("updown");
        if let (Some(x_tilt), Some(y_tilt)) = (x_tilt, y_tilt) {
            if let Some((entity, handle, player)) = (&entities, &handles, &mut player).join().next()
            {
                if player.state != PlayerState::Moving {
                    return;
                }
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
                        } else {
                            player.facing
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
                    player.facing = direction;
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
        dispatcher.add(PlayerAttackSystem, "player_attack", &[]);
        dispatcher.add(PlayerMovementSystem, "player_movement", &["player_attack"]);
        Ok(())
    }
}
