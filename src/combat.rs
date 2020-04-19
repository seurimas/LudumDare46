use crate::enemies::*;
use crate::physics::*;
use crate::player::*;
use crate::prelude::*;

#[derive(Debug)]
pub enum HitType {
    FriendlyAttack,
    EnemyAttack,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct AttackHitbox {
    pub id: usize,
    pub hit_type: HitType,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Health {
    pub friendly: bool,
    pub current_health: usize,
    pub last_attack: usize,
}

struct AttackHitboxSystem;

impl AttackHitboxSystem {
    fn hit_player(
        &self,
        physics: &mut Physics<f32>,
        health: &mut Health,
        player: &mut Player,
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
    }
    fn hit_goblin(
        &self,
        physics: &mut Physics<f32>,
        health: &mut Health,
        goblin: &mut Goblin,
        hitbox_physics: &PhysicsHandle,
        goblin_physics: &PhysicsHandle,
    ) {
        if let Some(direction) = physics.get_between(goblin_physics, hitbox_physics) {
            physics.set_velocity(
                goblin_physics,
                Direction::long_seek(direction).tilts() * -60.0,
            );
            goblin.state = GoblinState::Hit(0.5);
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
                    if health.last_attack != hitbox.id {
                        println!("New contact!");
                        println!("{:?} {:?}", health.friendly, hitbox.hit_type);
                        health.last_attack = hitbox.id;
                        if let Some(mut player) = players.get_mut(hit_entity) {
                            self.hit_player(
                                &mut physics,
                                &mut health,
                                &mut player,
                                &handle,
                                &hit_handle,
                            );
                        }
                        if let Some(mut goblin) = goblins.get_mut(hit_entity) {
                            self.hit_goblin(
                                &mut physics,
                                &mut health,
                                &mut goblin,
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

pub struct CombatBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for CombatBundle {
    fn build(
        self,
        _world: &mut World,
        dispatcher: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        dispatcher.add(AttackHitboxSystem, "attack_hitbox", &["physics"]);
        Ok(())
    }
}
