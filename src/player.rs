use crate::assets::SpriteStorage;
use crate::physics::*;
use amethyst::{
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

fn spawn_player(sprites: &SpriteStorage, player_builder: impl Builder) -> Entity {
    let shape = ShapeHandle::new(Ball::new(16.0));
    let body = RigidBodyDesc::new().status(BodyStatus::Dynamic).mass(100.0);
    let collider = ColliderDesc::new(shape);
    player_builder
        .with(PhysicsDesc::new(body, collider))
        .with(SpriteRender {
            sprite_sheet: sprites.ball_spritesheet.clone(),
            sprite_number: 0,
        })
        .with(Transform::default())
        .with(Player { walk_speed: 100.0 })
        .build()
}

pub fn spawn_player_world(world: &mut World) {
    let entities = world.entities();
    let update = world.write_resource::<LazyUpdate>();
    let builder = update.create_entity(&entities);
    let sprites = world.read_resource::<SpriteStorage>();
    let player = spawn_player(&sprites, builder);
    let builder = update.create_entity(&entities);
    initialize_camera(builder, player);
}

struct PlayerCameraSystem;
impl<'s> System<'s> for PlayerCameraSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        WriteStorage<'s, Camera>,
        Write<'s, ActiveCamera>,
        ReadStorage<'s, Player>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (mut transforms, mut cameras, mut active_camera, players, entities): Self::SystemData,
    ) {
        /*
                let mut player_location = Vector2::new(0.0, 0.0);
                for (_player, transform) in (&players, &transforms).join() {
                    player_location.x = transform.translation().x;
                    player_location.y = transform.translation().y;
                }
                let mut new_camera_transform = Transform::default();
                new_camera_transform.set_translation_xyz(player_location.x, player_location.y, 1.0);
                println!("{:?}", new_camera_transform.translation());
                if let Some((entity, _camera)) = (&entities, &cameras).join().next() {
                    //transforms.insert(entity, new_camera_transform);
                    entities.delete(entity);
                }
                let new_camera = entities.create();
                cameras.insert(new_camera, standard_camera());
                transforms.insert(new_camera, new_camera_transform);
                active_camera.entity = Some(new_camera);
        */
    }
}

struct PlayerMovementSystem;
impl<'s> System<'s> for PlayerMovementSystem {
    type SystemData = (
        Read<'s, InputHandler<StringBindings>>,
        Write<'s, Physics<f32>>,
        ReadStorage<'s, PhysicsHandle>,
        ReadStorage<'s, Player>,
    );

    fn run(&mut self, (input, mut physics, handles, player): Self::SystemData) {
        let x_tilt = input.axis_value("leftright");
        let y_tilt = input.axis_value("updown");
        if let (Some(x_tilt), Some(y_tilt)) = (x_tilt, y_tilt) {
            if let Some((handle, player)) = (&handles, &player).join().next() {
                physics.set_velocity(
                    handle,
                    Vector2::new(x_tilt * player.walk_speed, y_tilt * player.walk_speed),
                );
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
        dispatcher.add(PlayerCameraSystem, "player_camera", &[]);
        dispatcher.add(PlayerMovementSystem, "player_movement", &[]);
        Ok(())
    }
}
