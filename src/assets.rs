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
    utils::application_root_dir,
};
use serde::{Deserialize, Serialize};

pub fn get_resource(str: &str) -> String {
    format!(
        "{}/resources/{}",
        application_root_dir().unwrap().to_str().unwrap(),
        str
    )
}

pub fn load_texture<'a>(world: &mut World, path: String) -> Handle<Texture> {
    let loader = world.read_resource::<Loader>();
    let texture_storage = world.read_resource::<AssetStorage<Texture>>();
    loader.load(path, ImageFormat::default(), (), &texture_storage)
}
pub fn load_spritesheet<'a>(world: &mut World, path: String) -> SpriteSheetHandle {
    let texture_handle = load_texture(world, format!("{}.png", path));
    let loader = world.read_resource::<Loader>();
    let sprite_sheet_store = world.read_resource::<AssetStorage<SpriteSheet>>();
    loader.load(
        format!("{}.ron", path), // Here we load the associated ron file
        SpriteSheetFormat(texture_handle),
        (),
        &sprite_sheet_store,
    )
}

#[derive(Debug, Clone, Deserialize, PrefabData)]
pub struct MyPrefabData {
    sprite_scene: SpriteScenePrefab,
    animation_set: AnimationSetPrefab<AnimationId, SpriteRender>,
}

pub fn load_prefab(world: &mut World, path: String) -> Handle<Prefab<MyPrefabData>> {
    world.exec(|loader: PrefabLoader<'_, MyPrefabData>| loader.load(path, RonFormat, ()))
}

pub struct PrefabStorage {
    pub player: Handle<Prefab<MyPrefabData>>,
}

pub fn load_sound_file<'a>(world: &mut World, path: String) -> SourceHandle {
    let loader = world.read_resource::<Loader>();
    loader.load(path, WavFormat, (), &world.read_resource())
}

pub struct SpriteStorage {
    pub tile_spritesheet: SpriteSheetHandle,
}

pub struct SoundStorage {
    pub bounce_wav: SourceHandle,
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
}

#[derive(Eq, PartialOrd, PartialEq, Hash, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum AnimationId {
    Walk(Direction),
    Attack(Direction),
    Idle(Direction),
}

impl AnimationId {
    pub fn direction(&self) -> Direction {
        match self {
            AnimationId::Walk(direction) => *direction,
            AnimationId::Attack(direction) => *direction,
            AnimationId::Idle(direction) => *direction,
        }
    }

    pub fn is_attack(&self) -> bool {
        match self {
            AnimationId::Attack(_) => true,
            _ => false,
        }
    }
}
