extern crate nalgebra as na;
mod assets;
mod physics;
mod player;
mod prelude;
use amethyst::{
    assets::*,
    audio::{AudioBundle, SourceHandle, WavFormat},
    core::transform::*,
    ecs::*,
    prelude::*,
    renderer::{
        bundle::RenderingBundle,
        camera::*,
        debug_drawing::DebugLines,
        palette::Srgba,
        plugins::{RenderDebugLines, RenderFlat2D, RenderToWindow},
        sprite::SpriteSheetHandle,
        types::{DefaultBackend, Texture},
        ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat,
    },
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
};
use amethyst_imgui::RenderImgui;
use assets::*;
use imgui::*;
use na::{Isometry2, Point2, Point3, RealField, UnitQuaternion, Vector2, Vector3};
use ncollide2d::shape::*;
use nphysics2d::material::*;
use nphysics2d::object::*;
use physics::*;
use player::*;
use prelude::*;
use std::f32::consts::PI;

fn spawn_new_ball<'s>(sprite_sheet: SpriteSheetHandle, builder: impl Builder) {
    let (body, collider) = {
        let material = MaterialHandle::new(BasicMaterial::new(1.0, 0.01));
        let shape = ShapeHandle::new(Ball::new(16.0));
        (
            RigidBodyDesc::new()
                .position(Isometry2::new(Vector2::new(24.0, 256.0), 0.0))
                .angular_inertia(1.0)
                .mass(4.0),
            ColliderDesc::new(shape).material(material),
        )
    };
    builder
        .with(PhysicsDesc::new(body, collider))
        .with(SpriteRender {
            sprite_sheet,
            sprite_number: 0,
        })
        .build();
}

fn spawn_ball(world: &mut World) {
    let entities = world.entities();
    let update = world.write_resource::<LazyUpdate>();
    let builder = update.create_entity(&entities);
    let mut physics = world.write_resource::<Physics<f32>>();
    let sprites = world.read_resource::<SpriteStorage>();
    spawn_new_ball(sprites.ball_spritesheet.clone(), builder);
}

fn spawn_walls(world: &mut World) {
    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(256.0, 0.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(16.0, 512.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();
    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(-256.0, 0.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(16.0, 512.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();
    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(0.0, 256.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(512.0, 16.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();
    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(0.0, -256.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(512.0, 16.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();

    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(64.0, 64.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(16.0, 16.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();

    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(-64.0, 64.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(16.0, 16.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();

    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(64.0, -64.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(16.0, 16.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();

    let body = RigidBodyDesc::new()
        .position(Isometry2::new(Vector2::new(-64.0, -64.0), 0.0))
        .status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(Vector2::new(16.0, 16.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .build();
}

struct MyState;

impl SimpleState for MyState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.register::<PhysicsHandle>();

        let sprite_sheet = load_spritesheet(data.world, get_resource("Ball"));
        data.world.insert(SpriteStorage {
            ball_spritesheet: sprite_sheet,
        });
        let bounce_wav = load_sound_file(data.world, get_resource("bounce.wav"));
        data.world.insert(SoundStorage { bounce_wav });
        spawn_player_world(&mut data.world);
        spawn_walls(&mut data.world);
    }
}

struct ImguiDebugSystem {
    listbox_item_current: i32,
    box_current: i32,
}

impl Default for ImguiDebugSystem {
    fn default() -> Self {
        ImguiDebugSystem {
            listbox_item_current: 0,
            box_current: 0,
        }
    }
}
impl<'s> amethyst::ecs::System<'s> for ImguiDebugSystem {
    type SystemData = (
        Read<'s, FpsCounter>,
        Option<Read<'s, SpriteStorage>>,
        Write<'s, Physics<f32>>,
        Entities<'s>,
        ReadStorage<'s, PhysicsHandle>,
        Read<'s, LazyUpdate>,
    );
    fn run(&mut self, (fps, sprites, mut physics, entities, handles, update): Self::SystemData) {
        amethyst_imgui::with(|ui: &imgui::Ui| {
            let mut window = imgui::Window::new(im_str!("Test"));
            window.build(ui, || {
                ui.text(im_str!("This is a test!"));
                ui.text(im_str!("FPS: {}", fps.sampled_fps()));
                ui.separator();
                if ui.button(im_str!("New ball"), [50.0, 20.0]) {
                    if let Some(sprites) = sprites {
                        spawn_new_ball(
                            sprites.ball_spritesheet.clone(),
                            update.create_entity(&entities),
                        );
                    }
                }
                let mut items = Vec::new();
                let mut item_handles = Vec::new();
                let mut idx = 0;
                for (entity, handle) in (&entities, &handles).join() {
                    if let Some(body1) = physics.get_position(handle) {
                        items.push(im_str!("Item: {}", idx,));
                        item_handles.push(handle);
                        idx = idx + 1;
                    }
                }
                let mut base = Vec::with_capacity(items.len());
                for item in items.iter() {
                    base.push(item);
                }
                let items = base.as_slice();
                if ui.list_box(
                    im_str!("Test"),
                    &mut self.listbox_item_current,
                    items.as_ref(),
                    15,
                ) {
                    if let Some(handle) = item_handles.get(self.listbox_item_current as usize) {
                        physics.apply_impulse(handle, Vector2::new(0.0, 100.0));
                    }
                }
                let mut drag = ui.drag_int(im_str!("Box"), &mut self.box_current);
                if drag.build() {
                    for (handle) in (&handles).join() {
                        physics.set_rotation(
                            handle,
                            self.box_current as f32 / 360.0 as f32 * std::f32::consts::FRAC_PI_2,
                        );
                    }
                    println!("{}", self.box_current);
                }
            });
        });
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let resources_dir = app_root.join("resources");
    let display_config_path = resources_dir.join("display_config.ron");
    let input_path = get_resource("input.ron");

    let game_data = GameDataBuilder::default()
        .with_bundle(TransformBundle::new())?
        .with_bundle(
            amethyst::input::InputBundle::<amethyst::input::StringBindings>::new()
                .with_bindings_from_file(input_path)?,
        )?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?
                        .with_clear([0.0, 0.0, 0.0, 1.0]),
                )
                .with_plugin(RenderFlat2D::default())
                .with_plugin(RenderDebugLines::default())
                .with_plugin(RenderImgui::<amethyst::input::StringBindings>::default()),
        )?
        .with_bundle(AudioBundle::default())?
        .with_bundle(PhysicsBundle)?
        .with_bundle(PlayerBundle)?
        .with_bundle(FpsCounterBundle)?
        .with(DebugDrawShapes, "debug_shapes", &[])
        .with_barrier()
        .with(ImguiDebugSystem::default(), "imgui_demo", &[]);

    let mut game = Application::new(resources_dir, MyState, game_data)?;
    game.run();

    Ok(())
}

pub trait IsoConvert {
    fn pos2(&self) -> Point2<f32>;
    fn pos3(&self) -> Point3<f32>;
}

impl IsoConvert for Isometry2<f32> {
    fn pos2(&self) -> Point2<f32> {
        [self.translation.x as f32, self.translation.y as f32].into()
    }
    fn pos3(&self) -> Point3<f32> {
        [self.translation.x as f32, self.translation.y as f32, 0.0].into()
    }
}

struct DebugDrawShapes;

impl<'s> System<'s> for DebugDrawShapes {
    type SystemData = (Write<'s, DebugLines>, Read<'s, Physics<f32>>);

    fn run(&mut self, (mut debugLines, physics): Self::SystemData) {
        for (handle, collider) in physics.colliders.iter() {
            if let Some(circle) = collider.shape().as_shape::<Ball<f32>>() {
                debugLines.draw_circle(
                    collider.position().pos3(),
                    circle.radius() as f32,
                    16,
                    Srgba::new(1.0, 1.0, 1.0, 1.0),
                );
            } else if let Some(cube) = collider.shape().as_shape::<Cuboid<f32>>() {
                let pos = collider.position().pos2();
                let ext = cube.half_extents();
                debugLines.draw_rotated_rectangle(
                    [pos.x - ext.x as f32, pos.y - ext.y as f32].into(),
                    [pos.x + ext.x as f32, pos.y + ext.y as f32].into(),
                    0.0,
                    UnitQuaternion::new(Vector3::new(
                        0.0,
                        0.0,
                        collider.position().rotation.angle() as f32,
                    )),
                    Srgba::new(1.0, 1.0, 1.0, 1.0),
                );
            }
        }
    }
}
