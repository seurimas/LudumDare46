use crate::assets::{AnimationId, Direction, MyPrefabData, PrefabStorage};
use crate::combat::*;
use crate::physics::*;
use crate::prelude::*;
use amethyst::{
    animation::*,
    assets::{Handle, Prefab},
    core::{bundle::SystemBundle, timing::Time, transform::*},
    ecs::world::LazyBuilder,
    renderer::SpriteRender,
    ui::{UiText, UiTransform},
};
use na::Isometry2;
use ncollide2d::shape::*;
use nphysics2d::object::*;

fn spawn_crab(
    prefab: Handle<Prefab<MyPrefabData>>,
    player_builder: EntityBuilder,
    x: f32,
    y: f32,
    waypoint: &Entity,
) -> Entity {
    let shape = ShapeHandle::new(Ball::new(8.0));
    let body = RigidBodyDesc::new().status(BodyStatus::Dynamic).mass(1.0);
    let collider = ColliderDesc::new(shape);
    let mut transform = Transform::default();
    transform.set_translation_xyz(x, y, 1.0);
    player_builder
        .with(PhysicsDesc::new(body, collider))
        .with(prefab)
        .with(Goblin {
            walk_speed: 40.0,
            state: GoblinState::Idling(waypoint.clone(), 5.0),
            facing: Direction::South,
            attack_distance: 60.0,
            lunge_speed: 120.0,
            chase_distance: 60.0,
        })
        .with(Health::new(false, 3))
        .with(transform)
        .build()
}

fn spawn_goblin_attack_sensor(
    builder: LazyBuilder,
    goblin: Entity,
    direction: Direction,
) -> Entity {
    let offset = direction.tilts() * 6.0 + direction.clockwise().tilts() * 3.0;
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(6.0, 6.0)));
    let collider = ColliderDesc::new(shape)
        .sensor(true)
        .position(Isometry2::new(offset, 0.0));
    builder
        .with(AttachedSensor::new(collider))
        .with(AttackHitbox {
            id: rand::random(),
            hit_type: HitType::EnemyAttack,
            damage: 1,
        })
        .with(Parent { entity: goblin })
        .build()
}

fn spawn_spawner(player_builder: EntityBuilder, x: f32, y: f32, waypoint: &Entity) -> Entity {
    let mut transform = Transform::default();
    transform.set_translation_xyz(x, y, 1.0);
    player_builder
        .with(transform)
        .with(GoblinSpawner {
            waypoint: *waypoint,
        })
        .build()
}

fn spawn_waypoint(player_builder: EntityBuilder, x: f32, y: f32) -> Entity {
    let shape = ShapeHandle::new(Ball::new(4.0));
    let body = RigidBodyDesc::new().status(BodyStatus::Static);
    let collider = ColliderDesc::new(shape).sensor(true);
    let mut transform = Transform::default();
    transform.set_translation_xyz(x, y, 1.0);
    player_builder
        .with(PhysicsDesc::new(body, collider))
        .with(transform)
        .with(Waypoint {
            next: None,
            margin: 8.0,
        })
        .build()
}

pub fn spawn_waypoint_world(world: &mut World, x: f32, y: f32) -> Entity {
    spawn_waypoint(world.create_entity(), x, y)
}

pub fn spawn_spawner_world(world: &mut World, x: f32, y: f32, waypoint: &Entity) -> Entity {
    spawn_spawner(world.create_entity(), x, y, waypoint)
}

pub fn spawn_goblin_world(world: &mut World, x: f32, y: f32, waypoint: &Entity) -> Entity {
    let prefab = {
        let prefabs = world.read_resource::<PrefabStorage>();
        prefabs.crab.clone()
    };
    let builder = world.create_entity();
    spawn_crab(prefab, builder, x, y, waypoint)
}

#[derive(Debug, PartialEq)]
pub enum GoblinState {
    Idling(Entity, f32),
    Moving(Entity),
    Chasing(Entity, Entity),
    Attacking(Entity, usize, f32),
    Hit(Entity, f32),
}

impl GoblinState {
    pub fn get_waypoint(&self) -> Entity {
        match self {
            GoblinState::Idling(waypoint, _) => *waypoint,
            GoblinState::Moving(waypoint) => *waypoint,
            GoblinState::Chasing(waypoint, _) => *waypoint,
            GoblinState::Attacking(waypoint, _, _) => *waypoint,
            GoblinState::Hit(waypoint, _) => *waypoint,
        }
    }
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

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct GoblinSpawner {
    pub waypoint: Entity,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Waypoint {
    pub next: Option<Entity>,
    pub margin: f32,
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
        self.should_chase_name(
            &physics,
            &handles,
            &entities,
            &names,
            "pylon",
            &goblin,
            &goblin_handle,
        )
        .or_else(|| {
            self.should_chase_name(
                &physics,
                &handles,
                &entities,
                &names,
                "player",
                &goblin,
                &goblin_handle,
            )
        })
    }
    fn should_chase_name<'s>(
        &self,
        physics: &Physics<f32>,
        handles: &ReadStorage<'s, PhysicsHandle>,
        entities: &Entities<'s>,
        names: &ReadStorage<'s, Named>,
        name: &'static str,
        goblin: &Goblin,
        goblin_handle: &PhysicsHandle,
    ) -> Option<Entity> {
        get_named_entity(entities, names, name)
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
                    <= goblin.chase_distance * goblin.chase_distance
                {
                    Some(player_entity)
                } else {
                    None
                }
            })
    }
    fn get_waypoint<'s>(
        &self,
        physics: &Physics<f32>,
        goblin_handle: &PhysicsHandle,
        waypoint_handle: &PhysicsHandle,
        current: &Waypoint,
    ) -> Option<Entity> {
        if let Some(distance) = physics.get_between(waypoint_handle, goblin_handle) {
            if distance.x * distance.x + distance.y * distance.y < current.margin {
                current.next
            } else {
                None
            }
        } else {
            None
        }
    }

    fn walk(
        &self,
        direction: Direction,
        physics: &mut Physics<f32>,
        handle: &PhysicsHandle,
        goblin: &mut Goblin,
        control_set: &mut AnimationControlSet<AnimationId, SpriteRender>,
        animation_set: &AnimationSet<AnimationId, SpriteRender>,
    ) {
        goblin.facing = direction;
        physics.set_velocity(handle, goblin.facing.tilts() * goblin.walk_speed);
        set_active_animation(
            control_set,
            AnimationId::Walk(goblin.facing),
            &animation_set,
            EndControl::Loop(None),
            1.0,
        );
    }
}

impl<'s> System<'s> for GoblinAiSystem {
    type SystemData = (
        Write<'s, Physics<f32>>,
        Read<'s, Time>,
        ReadStorage<'s, Named>,
        ReadStorage<'s, Waypoint>,
        ReadStorage<'s, AttachedSensor>,
        ReadStorage<'s, Parent>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, AnimationSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, AnimationControlSet<AnimationId, SpriteRender>>,
        WriteStorage<'s, Goblin>,
        WriteStorage<'s, AttackHitbox>,
        Read<'s, LazyUpdate>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            mut physics,
            time,
            names,
            waypoints,
            sensors,
            parents,
            handles,
            animation_sets,
            mut control_sets,
            mut goblins,
            mut attacks,
            lazy,
            entities,
        ): Self::SystemData,
    ) {
        for (entity, handle, mut goblin) in (&entities, &handles, &mut goblins).join() {
            if let (Some(animation_set), Some(mut control_set)) = (
                animation_sets.get(entity),
                get_animation_set(&mut control_sets, entity),
            ) {
                match goblin.state {
                    GoblinState::Moving(target) => {
                        if let Some(player) = self
                            .should_chase(&physics, &handles, &entities, &names, &goblin, &handle)
                        {
                            goblin.state = GoblinState::Chasing(target, player);
                        } else {
                            self.walk(
                                goblin.facing,
                                &mut physics,
                                &handle,
                                &mut goblin,
                                &mut control_set,
                                &animation_set,
                            );
                            if let Some(waypoint_handle) = handles.get(target) {
                                if let Some(next) = self.get_waypoint(
                                    &physics,
                                    waypoint_handle,
                                    handle,
                                    waypoints.get(target).unwrap(),
                                ) {
                                    goblin.state = GoblinState::Moving(next);
                                } else {
                                    let offset = physics.get_between(handle, waypoint_handle);
                                    goblin.facing = Direction::short_seek(offset.unwrap(), 4.0);
                                }
                            }
                        }
                    }
                    GoblinState::Idling(waypoint, time_left) => {
                        for (_, sensor) in get_sensors(&entities, &sensors, &parents, entity).iter()
                        {
                            let sensor = *sensor;
                            if attacks.contains(sensor) {
                                lazy.exec(move |world| {
                                    world.delete_entity(sensor);
                                });
                            }
                        }
                        if let Some(player) = self
                            .should_chase(&physics, &handles, &entities, &names, &goblin, &handle)
                            .and_then(|player| if time_left < 2.0 { Some(player) } else { None })
                        {
                            goblin.state = GoblinState::Chasing(waypoint, player);
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
                                goblin.state = GoblinState::Moving(waypoint);
                            } else {
                                goblin.state =
                                    GoblinState::Idling(waypoint, time_left - time.delta_seconds());
                            }
                        }
                    }
                    GoblinState::Chasing(waypoint, player) => {
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
                                        goblin.state =
                                            GoblinState::Attacking(waypoint, rand::random(), 0.0);
                                        goblin.facing = direction;
                                        spawn_goblin_attack_sensor(
                                            lazy.create_entity(&entities),
                                            entity,
                                            goblin.facing,
                                        );
                                        found = true;
                                    }
                                }
                            }
                            if !found {
                                if let Some(offset) = physics.get_between(handle, player_handle) {
                                    self.walk(
                                        Direction::short_seek(offset, 4.0),
                                        &mut physics,
                                        &handle,
                                        &mut goblin,
                                        &mut control_set,
                                        &animation_set,
                                    );
                                }
                            }
                        } else {
                            goblin.state = GoblinState::Idling(waypoint, 4.0);
                        }
                    }
                    GoblinState::Attacking(waypoint, attack_id, progress) => {
                        if let Some(AnimationId::Attack(_)) = get_active_animation(control_set) {
                            if progress > 0.375 {
                                physics.set_velocity(
                                    handle,
                                    goblin.facing.tilts() * goblin.lunge_speed,
                                );
                            } else {
                                physics.set_velocity(handle, Vector2::zeros());
                            }
                            goblin.state = GoblinState::Attacking(
                                waypoint,
                                attack_id,
                                progress + time.delta_seconds(),
                            );
                        } else {
                            goblin.state = GoblinState::Idling(waypoint, 4.0);
                        }
                    }
                    GoblinState::Hit(waypoint, size) => {
                        if size < time.delta_seconds() {
                            goblin.state = GoblinState::Idling(waypoint, 3.0);
                        } else {
                            goblin.state = GoblinState::Hit(waypoint, size - time.delta_seconds());
                            set_active_animation(
                                control_set,
                                AnimationId::Staggered(goblin.facing),
                                &animation_set,
                                EndControl::Stay,
                                1.0,
                            );
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
pub struct WaveSystem {
    idle_time: f32,
    wave_num: usize,
}

impl<'s> System<'s> for WaveSystem {
    type SystemData = (
        ReadStorage<'s, Goblin>,
        Read<'s, LazyUpdate>,
        ReadStorage<'s, UiTransform>,
        WriteStorage<'s, UiText>,
        Read<'s, Time>,
        Entities<'s>,
    );
    fn run(&mut self, (goblins, lazy, transforms, mut ui_texts, time, entities): Self::SystemData) {
        let mut goblin_count = 0;
        for _goblin in (&goblins).join() {
            goblin_count += 1;
        }
        if goblin_count == 0 {
            self.idle_time += time.delta_seconds();
        } else {
            self.idle_time = 0.0;
        }
        for (transform, text) in (&transforms, &mut ui_texts).join() {
            if transform.id.eq("goblin_count") {
                if goblin_count > 0 {
                    text.text = format!("Goblins Left: {}", goblin_count);
                } else if self.idle_time < 15.0 {
                    text.text = format!("Next wave in: {}", (15 - self.idle_time as usize));
                }
            }
        }
        if self.idle_time > 15.0 {
            lazy.exec_mut(|world| {
                let spawners = world.exec(
                    |(entities, transforms, spawners): (
                        Entities<'_>,
                        ReadStorage<'_, Transform>,
                        ReadStorage<'_, GoblinSpawner>,
                    )| {
                        let mut spawn_list = Vec::new();
                        for (entity, transform, spawner) in
                            (&entities, &transforms, &spawners).join()
                        {
                            let translation = transform.translation();
                            spawn_list.push((translation.x, translation.y, spawner.waypoint));
                            println!("{:?}", spawn_list);
                        }
                        spawn_list
                    },
                );
                for (tx, ty, waypoint) in spawners.iter() {
                    println!("{} {}", tx, ty);
                    spawn_goblin_world(world, *tx, *ty, waypoint);
                }
            });
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
        dispatcher.add(
            WaveSystem {
                idle_time: 15.0,
                wave_num: 1,
            },
            "waves",
            &[],
        );
        dispatcher.add(GoblinAiSystem, "goblin", &[]);
        Ok(())
    }
}
