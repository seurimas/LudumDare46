extern crate nalgebra as na;
extern crate nalgebra19 as na19;
extern crate rand;
extern crate tiled;
mod assets;
mod combat;
mod enemies;
mod physics;
mod player;
mod prelude;
mod world;
use amethyst::{
    animation::AnimationBundle,
    assets::*,
    audio::{output::init_output, AudioBundle, SourceHandle, WavFormat},
    core::transform::*,
    ecs::*,
    input::{is_close_requested, is_key_down},
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
    tiles::{MortonEncoder, RenderTiles2D},
    ui::{RenderUi, UiBundle, UiCreator, UiEventType, UiFinder},
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
    winit::VirtualKeyCode,
};
use amethyst_imgui::RenderImgui;
use assets::*;
use combat::CombatBundle;
use enemies::*;
use imgui::*;
use na::{Isometry2, Point2, Point3, RealField, UnitQuaternion, Vector2, Vector3};
use ncollide2d::shape::*;
use nphysics2d::material::*;
use nphysics2d::object::*;
use physics::*;
use player::*;
use prelude::*;
use std::f32::consts::PI;
use world::*;

#[derive(Default)]
struct LoadingState {
    progress: Option<ProgressCounter>,
    assets: Option<(SpriteStorage, PrefabStorage, SoundStorage, MapStorage)>,
}

struct GameplayState {
    assets: (SpriteStorage, PrefabStorage, SoundStorage, MapStorage),
}
impl SimpleState for GameplayState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
        data.world.insert(WaveState {
            idle_time: 14.0,
            wave_num: 3,
        });
        data.world.insert(self.assets.0.clone());
        data.world.insert(self.assets.1.clone());
        data.world.insert(self.assets.2.clone());
        data.world.insert(self.assets.3.clone());
        initialize_tile_world(data.world);
        data.world.exec(|mut creator: UiCreator<'_>| {
            creator.create(get_resource("hud.ron"), ());
        });
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        let (entities, names): (Entities<'_>, ReadStorage<'_, Named>) = data.world.system_data();
        if get_named_entity(&entities, &names, "player").is_none() {
            return SimpleTrans::Switch(Box::new(MenuState {
                assets: self.assets.clone(),
                menu: "game_over.ron",
            }));
        }
        if get_named_entity(&entities, &names, "pylon").is_none() {
            return SimpleTrans::Switch(Box::new(MenuState {
                assets: self.assets.clone(),
                menu: "game_over.ron",
            }));
        }
        if data.world.read_resource::<WaveState>().wave_num == SPAWNS.len() {
            return SimpleTrans::Switch(Box::new(MenuState {
                assets: self.assets.clone(),
                menu: "game_over.ron",
            }));
        }
        SimpleTrans::None
    }
}

struct MenuState {
    assets: (SpriteStorage, PrefabStorage, SoundStorage, MapStorage),
    menu: &'static str,
}
impl SimpleState for MenuState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.delete_all();
        data.world.exec(|mut creator: UiCreator<'_>| {
            creator.create(get_resource(self.menu), ());
        });
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        match &event {
            StateEvent::Window(event) => {
                if is_close_requested(&event) {
                    Trans::Quit
                } else {
                    Trans::None
                }
            }
            StateEvent::Ui(ui_event) => data.world.exec(|finder: UiFinder<'_>| {
                if ui_event.event_type == UiEventType::Click {
                    if let Some(start) = finder.find("play") {
                        if start == ui_event.target {
                            return Trans::Push(Box::new(GameplayState {
                                assets: self.assets.clone(),
                            }));
                        }
                    }
                    if let Some(exit) = finder.find("exit") {
                        if exit == ui_event.target {
                            return Trans::Quit;
                        }
                    }
                }
                Trans::None
            }),
            _ => Trans::None,
        }
    }
}

impl SimpleState for LoadingState {
    fn on_start(&mut self, mut data: StateData<'_, GameData<'_, '_>>) {
        data.world.register::<PhysicsHandle>();
        data.world.insert(AssetStorage::<TiledMap>::default());

        init_output(data.world);

        let mut progress_counter = ProgressCounter::new();

        let tile_spritesheet =
            load_spritesheet(data.world, get_resource("Tiles"), &mut progress_counter);
        let player_prefab = load_prefab(
            data.world,
            get_resource("Player.ron"),
            &mut progress_counter,
        );
        let crab_prefab = load_prefab(
            data.world,
            get_resource("Enemies1.ron"),
            &mut progress_counter,
        );
        let goblin_hit = load_sound_file(
            data.world,
            get_resource("Goblin_Hit.wav"),
            &mut progress_counter,
        );
        let player_hit = load_sound_file(
            data.world,
            get_resource("Player_Hit.wav"),
            &mut progress_counter,
        );
        let pylon_hit = load_sound_file(
            data.world,
            get_resource("Pylon_Hit.wav"),
            &mut progress_counter,
        );
        let sword_slash = load_sound_file(
            data.world,
            get_resource("Sword_Slash.wav"),
            &mut progress_counter,
        );
        let main_theme = load_sound_file(
            data.world,
            get_resource("MainTheme.wav"),
            &mut progress_counter,
        );

        let village_map = load_map(
            data.world,
            get_resource("Village.tmx"),
            &mut progress_counter,
        );

        self.progress = Some(progress_counter);
        self.assets = Some((
            SpriteStorage { tile_spritesheet },
            PrefabStorage {
                player: player_prefab,
                crab: crab_prefab,
            },
            SoundStorage {
                goblin_hit,
                player_hit,
                pylon_hit,
                sword_slash,
                main_theme,
            },
            MapStorage { village_map },
        ));
    }

    fn update(&mut self, data: &mut StateData<GameData>) -> SimpleTrans {
        if let Some(progress) = &self.progress {
            println!("{:?}", progress);
            if progress.is_complete() {
                return SimpleTrans::Switch(Box::new(MenuState {
                    assets: self.assets.clone().unwrap(),
                    menu: "main_menu.ron",
                }));
            }
        }
        SimpleTrans::None
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
        .with_system_desc(
            PrefabLoaderSystemDesc::<MyPrefabData>::default(),
            "scene_loader",
            &[],
        )
        .with(DjSystem, "dj", &[])
        .with(Processor::<TiledMap>::new(), "tiled_map_processor", &[])
        .with_bundle(AnimationBundle::<AnimationId, SpriteRender>::new(
            "sprite_animation_control",
            "sprite_sampler_interpolation",
        ))?
        .with_bundle(
            TransformBundle::new()
                .with_dep(&["sprite_animation_control", "sprite_sampler_interpolation"]),
        )?
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
                .with_plugin(RenderTiles2D::<WorldTile, MortonEncoder>::default())
                .with_plugin(RenderDebugLines::default())
                .with_plugin(RenderUi::default())
                .with_plugin(RenderImgui::<amethyst::input::StringBindings>::default()),
        )?
        .with_bundle(AudioBundle::default())?
        .with_bundle(PhysicsBundle)?
        .with_bundle(PlayerBundle)?
        .with_bundle(EnemiesBundle)?
        .with_bundle(CombatBundle)?
        .with_bundle(FpsCounterBundle)?
        .with_bundle(UiBundle::<amethyst::input::StringBindings>::new())?;

    let mut game = Application::new(resources_dir, LoadingState::default(), game_data)?;
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
                    na19::geometry::Point3::<f32>::new(
                        collider.position().pos3().x,
                        collider.position().pos3().y,
                        collider.position().pos3().z,
                    ),
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
                    na19::UnitQuaternion::new(na19::Vector3::new(
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
