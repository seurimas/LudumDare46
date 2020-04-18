use amethyst::{
    assets::*,
    audio::{SourceHandle, WavFormat},
    ecs::*,
    prelude::*,
    renderer::{
        sprite::SpriteSheetHandle, types::Texture, ImageFormat, SpriteRender, SpriteSheet,
        SpriteSheetFormat,
    },
    utils::application_root_dir,
};

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
pub fn load_sound_file<'a>(world: &mut World, path: String) -> SourceHandle {
    let loader = world.read_resource::<Loader>();
    loader.load(path, WavFormat, (), &world.read_resource())
}

pub struct SpriteStorage {
    pub ball_spritesheet: SpriteSheetHandle,
}

pub struct SoundStorage {
    pub bounce_wav: SourceHandle,
}
