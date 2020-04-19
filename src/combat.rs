use crate::enemies::*;
use crate::physics::*;
use crate::player::*;
use crate::prelude::*;
use amethyst::ui::{UiFinder, UiTransform};

#[derive(Debug, PartialEq)]
pub enum HitType {
    FriendlyAttack,
    EnemyAttack,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct AttackHitbox {
    pub id: usize,
    pub hit_type: HitType,
    pub damage: usize,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Health {
    pub friendly: bool,
    pub current_health: usize,
    pub hit_by: Vec<usize>,
}

impl Health {
    pub fn new(friendly: bool, current_health: usize) -> Self {
        Health {
            friendly,
            current_health,
            hit_by: Vec::new(),
        }
    }
}

pub const MAX_PLAYER_HEALTH: usize = 16;
pub const MAX_PYLON_HEALTH: usize = 32;

struct AttackHitboxSystem;

impl AttackHitboxSystem {
    fn hit_pylon(
        &self,
        physics: &mut Physics<f32>,
        health: &mut Health,
        pylon: &mut Pylon,
        attack: &AttackHitbox,
        hitbox_physics: &PhysicsHandle,
        player_physics: &PhysicsHandle,
    ) {
        if health.current_health < attack.damage {
            health.current_health = 0;
        } else {
            health.current_health = health.current_health - attack.damage;
        }
    }
    fn hit_player(
        &self,
        physics: &mut Physics<f32>,
        health: &mut Health,
        player: &mut Player,
        attack: &AttackHitbox,
        hitbox_physics: &PhysicsHandle,
        player_physics: &PhysicsHandle,
    ) {
        if let Some(direction) = physics.get_between(player_physics, hitbox_physics) {
            physics.set_velocity(
                player_physics,
                Direction::long_seek(direction).tilts() * -60.0,
            );
            player.state = PlayerState::Hit(0.5);
        }
        if health.current_health < attack.damage {
            health.current_health = 0;
        } else {
            health.current_health = health.current_health - attack.damage;
        }
    }
    fn hit_goblin(
        &self,
        physics: &mut Physics<f32>,
        health: &mut Health,
        goblin: &mut Goblin,
        attack: &AttackHitbox,
        hitbox_physics: &PhysicsHandle,
        goblin_physics: &PhysicsHandle,
    ) {
        if let Some(direction) = physics.get_between(goblin_physics, hitbox_physics) {
            physics.set_velocity(
                goblin_physics,
                Direction::long_seek(direction).tilts() * -60.0,
            );
            goblin.state = GoblinState::Hit(goblin.state.get_waypoint(), 0.5);
        }
        if health.current_health < attack.damage {
            health.current_health = 0;
        } else {
            health.current_health = health.current_health - attack.damage;
        }
    }
}

impl<'s> System<'s> for AttackHitboxSystem {
    type SystemData = (
        Write<'s, Physics<f32>>,
        ReadStorage<'s, AttachedSensor>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Named>,
        WriteStorage<'s, Health>,
        WriteStorage<'s, Player>,
        WriteStorage<'s, Goblin>,
        WriteStorage<'s, Pylon>,
        ReadStorage<'s, AttackHitbox>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (
            mut physics,
            sensors,
            handles,
            names,
            mut healths,
            mut players,
            mut goblins,
            mut pylons,
            hitboxes,
            entities,
        ): Self::SystemData,
    ) {
        for (_entity, sensor, hitbox) in (&entities, &sensors, &hitboxes).join() {
            let handle = sensor.get_handle();
            for hit_entity in physics.get_intersections(&handle) {
                if let (Some(hit_handle), Some(mut health)) =
                    (handles.get(hit_entity), healths.get_mut(hit_entity))
                {
                    if !health.hit_by.contains(&hitbox.id)
                        && ((health.friendly && hitbox.hit_type == HitType::EnemyAttack)
                            || (!health.friendly && hitbox.hit_type == HitType::FriendlyAttack))
                    {
                        health.hit_by.push(hitbox.id);
                        if let Some(mut player) = players.get_mut(hit_entity) {
                            self.hit_player(
                                &mut physics,
                                &mut health,
                                &mut player,
                                &hitbox,
                                &handle,
                                &hit_handle,
                            );
                        }
                        if let Some(mut pylon) = pylons.get_mut(hit_entity) {
                            self.hit_pylon(
                                &mut physics,
                                &mut health,
                                &mut pylon,
                                &hitbox,
                                &handle,
                                &hit_handle,
                            );
                        }
                        if let Some(mut goblin) = goblins.get_mut(hit_entity) {
                            self.hit_goblin(
                                &mut physics,
                                &mut health,
                                &mut goblin,
                                &hitbox,
                                &handle,
                                &hit_handle,
                            );
                        }
                    }
                }
            }
        }
    }
}

struct NpcDeathSystem;
impl<'s> System<'s> for NpcDeathSystem {
    type SystemData = (Entities<'s>, ReadStorage<'s, Health>, Read<'s, LazyUpdate>);

    fn run(&mut self, (entities, healths, lazy): Self::SystemData) {
        for (entity, health) in (&entities, &healths).join() {
            if health.current_health == 0 {
                lazy.exec(move |world| {
                    world.delete_entity(entity);
                });
            }
        }
    }
}

const WIDTH_PER_HEALTH: f32 = 128.0 / MAX_PLAYER_HEALTH as f32;
const WIDTH_PER_PYLON: f32 = 128.0 / MAX_PYLON_HEALTH as f32;

pub struct HealthDisplaySystem;

impl<'s> System<'s> for HealthDisplaySystem {
    type SystemData = (
        ReadStorage<'s, Pylon>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, Health>,
        WriteStorage<'s, UiTransform>,
        Entities<'s>,
    );
    fn run(&mut self, (pylons, players, healths, mut transforms, entities): Self::SystemData) {
        if let Some((player, health)) = (&players, &healths).join().next() {
            for (transform) in (&mut transforms).join() {
                if transform.id.eq("full_health") {
                    transform.width = WIDTH_PER_HEALTH * health.current_health as f32;
                }
            }
        }
        if let Some((pylon, health)) = (&pylons, &healths).join().next() {
            for (transform) in (&mut transforms).join() {
                if transform.id.eq("full_pylon") {
                    transform.width = WIDTH_PER_HEALTH * health.current_health as f32;
                }
            }
        }
    }
}

pub struct CombatBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for CombatBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(HealthDisplaySystem, "health_bar", &[]);
        dispatcher.add(AttackHitboxSystem, "attack_hitbox", &["physics"]);
        dispatcher.add(NpcDeathSystem, "npc_death", &[]);
        Ok(())
    }
}
