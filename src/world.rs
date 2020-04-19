use crate::assets::{MapStorage, SpriteStorage, TiledMap};
use crate::physics::*;
use amethyst::{
    assets::{AssetStorage, Format},
    core::{bundle::SystemBundle, math::Point3, transform::*},
    ecs::*,
    error::Error,
    input::{InputHandler, StringBindings},
    prelude::*,
    renderer::{camera::*, SpriteRender},
    tiles::{MortonEncoder, Tile, TileMap},
};
use ncollide2d::shape::*;
use nphysics2d::object::*;

#[rustfmt::skip]
const TILES: &'static [usize] = &[
    0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 1, 0, 0, 0, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 1, 0, 0, 0, 1, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0,
];
const TILES_WIDTH: u32 = 9;

pub struct WorldTiles {
    layer0: Vec<Vec<tiled::LayerTile>>,
    map_size: (u32, u32),
    tile_size: (u32, u32),
}

impl WorldTiles {
    pub fn get_id(&self, x: usize, y: usize) -> Option<usize> {
        self.layer0
            .get(y)
            .and_then(|row| row.get(x))
            .and_then(|tile| {
                if tile.gid > 0 {
                    Some(tile.gid as usize - 1)
                } else {
                    None
                }
            })
    }
}

#[derive(Default, Clone)]
pub struct WorldTile;
impl Tile for WorldTile {
    fn sprite(&self, point: Point3<u32>, world: &World) -> Option<usize> {
        let world_tiles = world.read_resource::<WorldTiles>();
        let idx = (point.x + point.y * world_tiles.map_size.0) as usize;
        world_tiles.get_id(point.x as usize, point.y as usize)
    }
}

fn initialize_fence_post(world: &mut World, transform: Transform, parent: Entity) {
    let body = RigidBodyDesc::new().status(BodyStatus::Static);
    let shape = ShapeHandle::new(Cuboid::new(na::Vector2::new(16.0, 16.0)));
    let collider = ColliderDesc::new(shape);
    world
        .create_entity()
        .with(PhysicsDesc::new(body, collider))
        .with(transform)
        .with(Parent { entity: parent })
        .build();
}

fn get_map(world: &World) -> (WorldTiles, (u32, u32), (u32, u32)) {
    let maps = world.read_resource::<MapStorage>();
    let map_assets = world.read_resource::<AssetStorage<TiledMap>>();
    let tiled_map = map_assets.get(&maps.village_map).unwrap();
    let map_size = (tiled_map.0.width, tiled_map.0.height);
    let tile_size = (tiled_map.0.tile_width, tiled_map.0.tile_height);
    (
        WorldTiles {
            layer0: tiled_map.0.layers.get(0).unwrap().tiles.clone(),
            map_size,
            tile_size,
        },
        map_size,
        tile_size,
    )
}

pub fn initialize_tile_world(world: &mut World) {
    let (map, map_size, tile_size) = get_map(world);
    let tile_spritesheet = {
        let sprites = world.read_resource::<SpriteStorage>();
        sprites.tile_spritesheet.clone()
    };
    let map_entity = world
        .create_entity()
        .with(TileMap::<WorldTile, MortonEncoder>::new(
            na19::Vector3::new(map_size.0, map_size.1, 1),
            na19::Vector3::new(tile_size.0, tile_size.1, 1),
            Some(tile_spritesheet),
        ))
        .with(Transform::default())
        .build();
    for y in 0..map_size.1 {
        for x in 0..map_size.0 {
            if map.get_id(x as usize, y as usize) == Some(1) {
                let mut transform = Transform::default();
                let x = (x * tile_size.0) as f32 - (tile_size.0 as f32 * map_size.0 as f32 / 2.0);
                let y =
                    ((y + 1) * tile_size.1) as f32 - (tile_size.1 as f32 * map_size.1 as f32 / 2.0);
                transform.set_translation_xyz(x, y, 0.0);
                initialize_fence_post(world, transform, map_entity);
            }
        }
    }
    world.insert::<WorldTiles>(map);
}
