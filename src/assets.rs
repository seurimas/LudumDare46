use amethyst::{
    animation::*,
    assets::*,
    audio::{SourceHandle, WavFormat},
    derive::PrefabData,
    ecs::*,
    error::Error,
    prelude::*,
    renderer::{
        sprite::{prefab::SpriteScenePrefab, SpriteSheetHandle},
        types::Texture,
        ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat,
    },
    utils::{application_root_dir, scene::BasicScenePrefab},
};
use na::Vector2;
use serde::{Deserialize, Serialize};

pub fn get_resource(str: &str) -> String {
    format!(
        "{}/resources/{}",
        application_root_dir().unwrap().to_str().unwrap(),
        str
    )
}

pub fn load_texture<'a>(
    world: &mut World,
    path: String,
    progress: &'a mut ProgressCounter,
) -> Handle<Texture> {
    let loader = world.read_resource::<Loader>();
    let texture_storage = world.read_resource::<AssetStorage<Texture>>();
    loader.load(path, ImageFormat::default(), progress, &texture_storage)
}
pub fn load_spritesheet<'a>(
    world: &mut World,
    path: String,
    progress: &'a mut ProgressCounter,
) -> SpriteSheetHandle {
    let texture_handle = load_texture(world, format!("{}.png", path), progress);
    let loader = world.read_resource::<Loader>();
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    loader.load(
        format!("{}.ron", path), // Here we load the associated ron file
        SpriteSheetFormat(texture_handle),
        progress,
        &sprite_sheet_store,
    )
}

#[derive(Debug, Clone, Deserialize, PrefabData)]
pub struct MyPrefabData {
    sprite_scene: SpriteScenePrefab,
    animation_set: AnimationSetPrefab<AnimationId, SpriteRender>,
}

pub fn load_prefab<'a>(
    world: &mut World,
    path: String,
    progress: &'a mut ProgressCounter,
) -> Handle<Prefab<MyPrefabData>> {
    world.exec(|loader: PrefabLoader<'_, MyPrefabData>| loader.load(path, RonFormat, progress))
}

#[derive(Clone)]
pub struct PrefabStorage {
    pub player: Handle<Prefab<MyPrefabData>>,
    pub crab: Handle<Prefab<MyPrefabData>>,
}

pub fn load_sound_file<'a>(
    world: &mut World,
    path: String,
    progress: &'a mut ProgressCounter,
) -> SourceHandle {
    let loader = world.read_resource::<Loader>();
    loader.load(path, WavFormat, (), &world.read_resource())
}

#[derive(Clone)]
pub struct SpriteStorage {
    pub tile_spritesheet: SpriteSheetHandle,
}

#[derive(Clone)]
pub struct SoundStorage {
    pub goblin_hit: SourceHandle,
    pub player_hit: SourceHandle,
    pub pylon_hit: SourceHandle,
    pub sword_slash: SourceHandle,
    pub main_theme: SourceHandle,
}

#[derive(Clone, Debug)]
pub struct TiledMap(pub tiled::Map);

#[derive(Clone, Copy, Debug, Default)]
pub struct TiledFormat;

impl Format<tiled::Map> for TiledFormat {
    fn name(&self) -> &'static str {
        "TiledFormat"
    }

    fn import_simple(&self, bytes: Vec<u8>) -> Result<tiled::Map, Error> {
        println!("LOADING FORMAT");
        let reader = bytes.as_slice();
        Ok(tiled::parse(reader).unwrap())
    }
}

impl Asset for TiledMap {
    const NAME: &'static str = "tiled::Map";
    // use `Self` if the type is directly serialized.
    type Data = tiled::Map;
    type HandleStorage = VecStorage<Handle<Self>>;
}

impl ProcessableAsset for TiledMap {
    fn process(tiled_map: Self::Data) -> Result<ProcessingState<Self>, Error> {
        Ok(ProcessingState::Loaded(Self(tiled_map)))
    }
}

#[derive(Clone)]
pub struct MapStorage {
    pub village_map: Handle<TiledMap>,
}

pub fn load_map<'a>(
    world: &mut World,
    path: String,
    progress: &'a mut ProgressCounter,
) -> Handle<TiledMap> {
    println!("LOADING MAP");
    let loader = world.read_resource::<Loader>();
    let map_storage = world.read_resource::<AssetStorage<TiledMap>>();
    loader.load(path, TiledFormat::default(), progress, &map_storage)
}

#[derive(Eq, PartialOrd, PartialEq, Hash, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum Direction {
    East,
    North,
    West,
    South,
}

impl Direction {
    pub fn vec() -> Vec<Self> {
        vec![
            Direction::East,
            Direction::North,
            Direction::West,
            Direction::South,
        ]
    }

    pub fn tilts(&self) -> Vector2<f32> {
        match self {
            Direction::East => Vector2::new(1.0, 0.0),
            Direction::North => Vector2::new(0.0, 1.0),
            Direction::West => Vector2::new(-1.0, 0.0),
            Direction::South => Vector2::new(0.0, -1.0),
        }
    }

    pub fn clockwise(&self) -> Self {
        match self {
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
            Direction::North => Direction::East,
        }
    }

    pub fn pick() -> Self {
        if rand::random() {
            if rand::random() {
                Direction::East
            } else {
                Direction::North
            }
        } else {
            if rand::random() {
                Direction::West
            } else {
                Direction::South
            }
        }
    }

    pub fn short_seek(offset: Vector2<f32>, margin: f32) -> Self {
        if f32::abs(offset.x) < margin || f32::abs(offset.y) < margin {
            Direction::long_seek(offset)
        } else if f32::abs(offset.x) > f32::abs(offset.y) {
            if offset.y > 0.0 {
                Direction::North
            } else {
                Direction::South
            }
        } else {
            if offset.x > 0.0 {
                Direction::East
            } else {
                Direction::West
            }
        }
    }

    pub fn long_seek(offset: Vector2<f32>) -> Self {
        if f32::abs(offset.x) < f32::abs(offset.y) {
            if offset.y > 0.0 {
                Direction::North
            } else {
                Direction::South
            }
        } else {
            if offset.x > 0.0 {
                Direction::East
            } else {
                Direction::West
            }
        }
    }
}

#[derive(Eq, PartialOrd, PartialEq, Hash, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum AnimationId {
    Walk(Direction),
    Attack(Direction),
    Idle(Direction),
    Staggered(Direction),
}

impl AnimationId {
    pub fn direction(&self) -> Direction {
        match self {
            AnimationId::Walk(direction) => *direction,
            AnimationId::Attack(direction) => *direction,
            AnimationId::Idle(direction) => *direction,
            AnimationId::Staggered(direction) => *direction,
        }
    }

    pub fn is_attack(&self) -> bool {
        match self {
            AnimationId::Attack(_) => true,
            _ => false,
        }
    }
}
