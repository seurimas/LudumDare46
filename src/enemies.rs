use crate::assets::{AnimationId, Direction, PrefabStorage};
use crate::combat::*;
use crate::physics::*;
use crate::prelude::*;
use amethyst::{
    animation::*,
    core::{bundle::SystemBundle, timing::Time, transform::*},
    ecs::world::LazyBuilder,
    renderer::SpriteRender,
};
use ncollide2d::shape::*;
use nphysics2d::object::*;

fn spawn_crab(prefabs: &PrefabStorage, player_builder: LazyBuilder) -> Entity {
    let shape = ShapeHandle::new(Ball::new(8.0));
    let body = RigidBodyDesc::new().status(BodyStatus::Dynamic).mass(10.0);
    let collider = ColliderDesc::new(shape);
    let mut transform = Transform::default();
    transform.set_translation_xyz(0.0, 64.0, 1.0);
    player_builder
        .with(PhysicsDesc::new(body, collider))
        .with(prefabs.crab.clone())
        .with(Goblin {
            walk_speed: 80.0,
            state: GoblinState::Idling(5.0),
            facing: Direction::South,
            attack_distance: 100.0,
            lunge_speed: 120.0,
            chase_distance: 150.0,
        })
        .with(Health {
            friendly: false,
            current_health: 100,
            last_attack: 0,
        })
        .with(transform)
        .build()
}

fn spawn_attack_sensor(builder: LazyBuilder, enemy: Entity) -> Entity {
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(8.0, 8.0)));
    let collider = ColliderDesc::new(shape).sensor(true);
    builder
        .with(AttachedSensor::new(collider))
        .with(Parent { entity: enemy })
        .build()
}

pub fn spawn_crab_world(world: &mut World) {
    let entities = world.entities();
    let update = world.write_resource::<LazyUpdate>();
    let builder = update.create_entity(&entities);
    let prefabs = world.read_resource::<PrefabStorage>();
    let crab = spawn_crab(&prefabs, builder);
    let builder = update.create_entity(&entities);
    spawn_attack_sensor(builder, crab);
}

#[derive(Debug, PartialEq)]
pub enum GoblinState {
    Idling(f32),
    Moving(f32),
    Chasing(Entity),
    Attacking(usize, f32),
    Hit,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Goblin {
    pub walk_speed: f32,
    pub lunge_speed: f32,
    pub state: GoblinState,
    pub facing: Direction,
    pub chase_distance: f32,
    pub attack_distance: f32,
}

struct GoblinAiSystem;
impl GoblinAiSystem {
    fn should_chase<'s>(
        &self,
        physics: &Physics<f32>,
        handles: &ReadStorage<'s, PhysicsHandle>,
        entities: &Entities<'s>,
        names: &ReadStorage<'s, Named>,
        goblin: &Goblin,
        goblin_handle: &PhysicsHandle,
    ) -> Option<Entity> {
        get_named_entity(entities, names, "player")
            .and_then(|player_entity| {
                handles
                    .get(player_entity)
                    .map(|handle| (player_entity, handle))
            })
            .and_then(|(player_entity, player_handle)| {
                physics
                    .get_between(player_handle, goblin_handle)
                    .map(|offset| (player_entity, offset))
            })
            .and_then(|(player_entity, offset)| {
                if offset.x * offset.x + offset.y * offset.y
                    <= goblin.attack_distance * goblin.attack_distance
                {
                    Some(player_entity)
                } else {
                    None
                }
            })
    }
}

impl<'s> System<'s> for GoblinAiSystem {
    type SystemData = (
        Write<'s, Physics<f32>>,
        Read<'s, Time>,
        ReadStorage<'s, Named>,
        ReadStorage<'s, AttachedSensor>,
        ReadStorage<'s, Parent>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, AnimationSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, Goblin>,
        WriteStorage<'s, AttackHitbox>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            mut physics,
            time,
            names,
            sensors,
            parents,
            handles,
            animation_sets,
            mut control_sets,
            mut goblins,
            mut attacks,
            entities,
        ): Self::SystemData,
    ) {
        for (entity, handle, goblin) in (&entities, &handles, &mut goblins).join() {
            if let (Some(animation_set), Some(control_set)) = (
                animation_sets.get(entity),
                get_animation_set(&mut control_sets, entity),
            ) {
                for (_, sensor) in get_sensors(&entities, &sensors, &parents, entity) {
                    attacks.remove(sensor);
                }
                match goblin.state {
                    GoblinState::Moving(time_left) => {
                        if let Some(player) = self
                            .should_chase(&physics, &handles, &entities, &names, &goblin, &handle)
                        {
                            goblin.state = GoblinState::Chasing(player);
                        } else {
                            physics.set_velocity(handle, goblin.facing.tilts() * goblin.walk_speed);
                            set_active_animation(
                                control_set,
                                AnimationId::Walk(goblin.facing),
                                &animation_set,
                                EndControl::Loop(None),
                                1.0,
                            );
                            if time_left < time.delta_seconds() {
                                goblin.state = GoblinState::Idling(2.0);
                            } else {
                                goblin.state =
                                    GoblinState::Moving(time_left - time.delta_seconds());
                            }
                        }
                    }
                    GoblinState::Idling(time_left) => {
                        if let Some(player) = self
                            .should_chase(&physics, &handles, &entities, &names, &goblin, &handle)
                        {
                            goblin.state = GoblinState::Chasing(player);
                        } else {
                            physics.set_velocity(handle, Vector2::zeros());
                            set_active_animation(
                                control_set,
                                AnimationId::Idle(goblin.facing),
                                &animation_set,
                                EndControl::Loop(None),
                                1.0,
                            );
                            if time_left < time.delta_seconds() {
                                goblin.state = GoblinState::Moving(2.0);
                                goblin.facing = Direction::pick();
                            } else {
                                goblin.state =
                                    GoblinState::Idling(time_left - time.delta_seconds());
                            }
                        }
                    }
                    GoblinState::Chasing(player) => {
                        println!("CHASING");
                        if let Some(player_handle) = handles.get(player) {
                            let mut found = false;
                            for direction in Direction::vec() {
                                for (seen, distance) in
                                    physics.ray_cast(handle, direction.tilts(), None).iter()
                                {
                                    if *seen == player && *distance < goblin.attack_distance {
                                        set_active_animation(
                                            control_set,
                                            AnimationId::Attack(direction),
                                            &animation_set,
                                            EndControl::Stay,
                                            1.0,
                                        );
                                        goblin.state = GoblinState::Attacking(rand::random(), 0.0);
                                        goblin.facing = direction;
                                        println!("Attacking {:?}", direction);
                                        found = true;
                                    }
                                }
                            }
                            if !found {
                                if let Some(offset) = physics.get_between(handle, player_handle) {
                                    goblin.facing = Direction::short_seek(offset);

                                    physics.set_velocity(
                                        handle,
                                        goblin.facing.tilts() * goblin.walk_speed,
                                    );
                                    set_active_animation(
                                        control_set,
                                        AnimationId::Walk(goblin.facing),
                                        &animation_set,
                                        EndControl::Loop(None),
                                        1.0,
                                    );
                                }
                            }
                        } else {
                            goblin.state = GoblinState::Idling(2.0);
                        }
                    }
                    GoblinState::Attacking(attack_id, progress) => {
                        if let Some(AnimationId::Attack(_)) = get_active_animation(control_set) {
                            if progress > 0.375 {
                                physics.set_velocity(
                                    handle,
                                    goblin.facing.tilts() * goblin.lunge_speed,
                                );
                                for (_, sensor) in
                                    get_sensors(&entities, &sensors, &parents, entity)
                                {
                                    attacks.insert(
                                        sensor,
                                        AttackHitbox {
                                            id: attack_id,
                                            hit_type: HitType::EnemyAttack,
                                        },
                                    );
                                    if let Some(sensor) = sensors.get(sensor) {
                                        let offset = goblin.facing.tilts() * 4.0
                                            + goblin.facing.clockwise().tilts() * 4.0;
                                        physics.set_sensor_position(sensor, offset.x, offset.y);
                                    }
                                }
                            } else {
                                physics.set_velocity(handle, Vector2::zeros());
                            }
                            goblin.state =
                                GoblinState::Attacking(attack_id, progress + time.delta_seconds());
                        } else {
                            goblin.state = GoblinState::Idling(2.0);
                        }
                    }
                    _ => {}
                }
            } else {
                println!("NO ANIMATION SET OR CONTROLLER");
            }
        }
    }
}

pub struct EnemiesBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for EnemiesBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(GoblinAiSystem, "goblin", &[]);
        Ok(())
    }
}
