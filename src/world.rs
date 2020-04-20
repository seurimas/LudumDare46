use crate::assets::{MapStorage, SpriteStorage, TiledMap};
use crate::enemies::{spawn_spawner_world, spawn_waypoint_world, Waypoint};
use crate::physics::*;
use crate::player::{spawn_player_world, spawn_pylon_world};
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

fn get_map(
    world: &World,
) -> (
    WorldTiles,
    tiled::Tileset,
    WorldTiles,
    (u32, u32),
    (u32, u32),
) {
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
        tiled_map.0.tilesets.get(0).unwrap().clone(),
        WorldTiles {
            layer0: tiled_map.0.layers.get(1).unwrap().tiles.clone(),
            map_size,
            tile_size,
        },
        map_size,
        tile_size,
    )
}

fn tile_type(tileset: &tiled::Tileset, tile_id: usize) -> String {
    for tile in tileset.tiles.iter() {
        if tile.id == tile_id as u32 {
            if let Some(tile_type) = &tile.tile_type {
                return tile_type.to_string();
            } else {
                return "".to_string();
            }
        }
    }
    "".to_string()
}

fn is_fence(tileset: &tiled::Tileset, tile_id: usize) -> bool {
    tile_type(tileset, tile_id).eq("f")
}

fn closest_waypoint(
    tx: f32,
    ty: f32,
    waypoints: &Vec<(f32, f32, usize)>,
    of_type: Option<usize>,
) -> (usize, f32) {
    let mut best = 0;
    let mut best_distance = 999999999999.0;
    for i in 0..waypoints.len() {
        let (wx, wy, wt) = waypoints.get(i).unwrap();
        let dx = tx - wx;
        let dy = ty - wy;
        let distance = dx * dx + dy * dy;
        if let Some(waypoint_type) = of_type {
            if *wt != waypoint_type - 1 {
                continue;
            }
        }
        if distance < best_distance {
            best = i;
            best_distance = distance;
        }
    }
    (best, best_distance)
}

fn follow_waypoints(
    world: &mut World,
    waypoints: &Vec<(f32, f32, usize)>,
    waypoint_entities: &Vec<Entity>,
) {
    world.exec(|mut waypoints_store: WriteStorage<'_, Waypoint>| {
        for i in 0..waypoints.len() {
            let ent = waypoint_entities.get(i).unwrap();
            let (wx, wy, wt) = waypoints.get(i).unwrap();
            if *wt > 1 {
                let (best, distance) = closest_waypoint(*wx, *wy, waypoints, Some(*wt));
                println!("{} to {}", i, best);
                if let Some(mut waypoint_obj) = waypoints_store.get_mut(*ent) {
                    println!("{}", distance);
                    waypoint_obj.next = waypoint_entities.get(best).cloned();
                }
            }
        }
    });
}

pub fn initialize_tile_world(world: &mut World) {
    let (map, tileset, obj_map, map_size, tile_size) = get_map(world);
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
    let mut waypoints = Vec::new();
    let mut goblins = Vec::new();
    let mut pylons = Vec::new();
    for y in 0..map_size.1 {
        for x in 0..map_size.0 {
            let tx = (x * tile_size.0) as f32 - (tile_size.0 as f32 * map_size.0 as f32 / 2.0);
            let ty = (tile_size.1 as f32 * map_size.1 as f32 / 2.0) - (y * tile_size.1) as f32;
            if let Some(tile_id) = map.get_id(x as usize, y as usize) {
                if is_fence(&tileset, tile_id) {
                    let mut transform = Transform::default();
                    transform.set_translation_xyz(tx, ty, 0.0);
                    initialize_fence_post(world, transform, map_entity);
                }
            }
            if let Some(obj_id) = obj_map.get_id(x as usize, y as usize) {
                match tile_type(&tileset, obj_id).as_ref() {
                    "Goblin" => {
                        goblins.push((tx, ty));
                    }
                    "Waypoint1" => {
                        waypoints.push((tx, ty, 1));
                    }
                    "Waypoint2" => {
                        waypoints.push((tx, ty, 2));
                    }
                    "Waypoint3" => {
                        waypoints.push((tx, ty, 3));
                    }
                    "Waypoint4" => {
                        waypoints.push((tx, ty, 4));
                    }
                    "Waypoint5" => {
                        waypoints.push((tx, ty, 5));
                    }
                    "Pylon" => {
                        pylons.push((tx, ty));
                    }
                    "Player" => {
                        spawn_player_world(world, tx, ty);
                    }
                    _ => {}
                }
            }
        }
    }
    world.insert::<WorldTiles>(map);
    let pylon = spawn_pylon_world(world, pylons.get(0).unwrap().0, pylons.get(0).unwrap().1);
    let mut waypoint_entities: Vec<Entity> = waypoints
        .clone()
        .into_iter()
        .map(|(tx, ty, _)| spawn_waypoint_world(world, tx, ty))
        .collect();
    world.maintain();
    follow_waypoints(world, &waypoints, &waypoint_entities);
    let mut goblin_entities: Vec<Entity> = goblins
        .into_iter()
        .map(move |(tx, ty)| {
            spawn_spawner_world(
                world,
                tx,
                ty,
                waypoint_entities
                    .get(closest_waypoint(tx, ty, &waypoints, None).0)
                    .unwrap(),
            )
        })
        .collect();
}
