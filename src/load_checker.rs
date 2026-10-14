// bevy_geotilemap: Library to create geotilemaps in the Bevy game engine for Rust.
// Copyright (C) 2024 INVAP S.E.
//
// This file is part of bevy_geotilemap.
//
// Licensed under either of:
//
// * MIT License
// * Apache License, Version 2.0
//
// at your option.
//
// Unless you explicitly state otherwise, any contribution intentionally
// submitted for inclusion in the work by you shall be dual licensed as above,
// without any additional terms or conditions.

use std::{collections::HashMap, ops::Range};

use bevy::{
    asset::{AssetServer, Handle},
    ecs::{component::Component, entity::Entity, resource::Resource},
    image::Image,
};

#[cfg(test)]
use mockall::automock;

#[derive(Component, Eq, Hash, PartialEq, Clone, Debug)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub geotilemap: Entity,
}

impl Tile {
    pub fn new(x: u32, y: u32, z: u32, geotilemap: Entity) -> Self {
        Self {
            x,
            y,
            z,
            geotilemap,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileAsset {
    pub image: Handle<Image>,
    pub target_entity: Entity,
    pub ready_to_despawn_countdown: usize,
}

#[cfg_attr(test, automock)]
pub trait LoadChecker {
    fn asset_is_ready(&self, image: &Handle<Image>) -> bool;
}

#[derive(Debug, Clone)]
pub struct TileLoadCheck {
    pub asset_server: AssetServer,
}

impl LoadChecker for TileLoadCheck {
    fn asset_is_ready(&self, image: &Handle<Image>) -> bool {
        self.asset_server
            .get_recursive_dependency_load_state(image)
            .is_some_and(|state| state.is_loaded())
    }
}

// A resource that holds the current loading tiles
#[derive(Resource, Debug, Clone)]
pub struct TileDependencyTree<T: LoadChecker> {
    // Holds tile image handles
    pub handle_hash: HashMap<Tile, TileAsset>,
    // Number of frames that everything needs to be ready for.
    // This is to prevent going into the fully loaded state in instances
    // where there might be a some frames between certain loading/pipelines action.
    pub confirmation_frames: usize,
    load_checker: T,
}

impl<T: LoadChecker> TileDependencyTree<T> {
    pub fn new(confirmation_frames: usize, load_checker: T) -> Self {
        Self {
            handle_hash: HashMap::new(),
            confirmation_frames,
            load_checker,
        }
    }

    pub fn new_tile(&mut self, tile: &Tile, image: Handle<Image>, target_entity: Entity) {
        self.handle_hash.insert(
            tile.clone(),
            TileAsset {
                image,
                target_entity,
                ready_to_despawn_countdown: 0,
            },
        );
    }

    pub fn parent_ready(&self, tile: &Tile, zoom_diff: u32) -> bool {
        //return true;
        if tile.z == 0 {
            return false;
        }

        let parent_tile = Tile::new(
            tile.x / 2_u32.pow(zoom_diff),
            tile.y / 2_u32.pow(zoom_diff),
            tile.z - zoom_diff,
            tile.geotilemap,
        );

        self.handle_hash
            .get(&parent_tile)
            .is_some_and(|tile_asset| self.load_checker.asset_is_ready(&tile_asset.image))
    }

    pub fn children_ready(
        &self,
        tile: &Tile,
        zoom_diff: u32,
        x_range: Range<u32>,
        y_range: Range<u32>,
    ) -> bool {
        //return true;
        (x_range)
            .flat_map(|x| y_range.clone().map(move |y| (x, y)))
            .all(|(x, y)| {
                if Self::is_child(tile, x, y, zoom_diff) {
                    let children_tile = Tile::new(x, y, tile.z + zoom_diff, tile.geotilemap);

                    self.handle_hash
                        .get(&children_tile)
                        .is_some_and(|tile_asset| {
                            self.load_checker.asset_is_ready(&tile_asset.image)
                        })
                } else {
                    true
                }
            })
    }

    fn is_child(tile: &Tile, x: u32, y: u32, zoom_diff: u32) -> bool {
        let tile_axis_subdivision_count = 2_u32.pow(zoom_diff);
        let x_min = tile.x * tile_axis_subdivision_count;
        let x_max = x_min + tile_axis_subdivision_count;
        let y_min = tile.y * tile_axis_subdivision_count;
        let y_max = y_min + tile_axis_subdivision_count;

        x >= x_min && x < x_max && y >= y_min && y < y_max
    }
}

#[cfg(test)]
mod child_tests {
    use std::marker::PhantomData;

    use bevy::{
        asset::{Handle, uuid::Uuid},
        ecs::entity::{Entity, EntityIndex},
        image::Image,
    };
    use rstest::rstest;

    use crate::load_checker::{MockLoadChecker, Tile, TileDependencyTree};

    fn entity(i: u32) -> Entity {
        Entity::from_index(EntityIndex::from_raw_u32(i).unwrap())
    }

    fn create_tile(x: u32, y: u32, z: u32) -> Tile {
        Tile::new(x, y, z, entity(0))
    }

    #[rstest]
    fn all_children_ready_zoom_diff_1() {
        let mut checker = MockLoadChecker::new();
        checker.expect_asset_is_ready().times(4).return_const(true);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(0, 0, 0);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 1, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(1, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(1, 1, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        assert!(loading_tiles.children_ready(&target_tile, 1, 0..2, 0..2))
    }

    #[rstest]
    fn one_children_not_ready_zoom_diff_1() {
        let mut checker = MockLoadChecker::new();
        checker
            .expect_asset_is_ready()
            .times(3)
            .return_const(true)
            .once()
            .return_const(false);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(0, 0, 0);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 1, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(1, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(1, 1, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        assert!(!loading_tiles.children_ready(&target_tile, 1, 0..2, 0..2))
    }

    #[rstest]
    fn one_children_not_spawned_zoom_diff_1() {
        let mut checker = MockLoadChecker::new();
        checker.expect_asset_is_ready().times(3).return_const(true);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(0, 0, 0);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 1, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(1, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        assert!(!loading_tiles.children_ready(&target_tile, 1, 0..2, 0..2))
    }

    #[rstest]
    fn all_children_ready_zoom_diff_1_in_view() {
        let mut checker = MockLoadChecker::new();
        checker.expect_asset_is_ready().times(1).return_const(true);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(0, 0, 0);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 1, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(1, 0, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(1, 1, 1),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        assert!(loading_tiles.children_ready(&target_tile, 1, 1..4, 1..4))
    }

    #[rstest]
    #[case(2)]
    #[case(3)]
    #[case(4)]
    #[case(5)]
    #[case(6)]
    #[case(7)]
    fn all_children_ready_zoom_diff_greater_than_1(#[case] zoom_level: u32) {
        let mut checker = MockLoadChecker::new();
        let row_count = 2_usize.pow(zoom_level);
        let count_child_tiles = row_count * row_count;

        checker
            .expect_asset_is_ready()
            .times(count_child_tiles)
            .return_const(true);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(0, 0, 0);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        for x in 0..row_count {
            for y in 0..row_count {
                loading_tiles.new_tile(
                    &create_tile(x as u32, y as u32, zoom_level),
                    Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
                    entity(1),
                );
            }
        }

        assert!(loading_tiles.children_ready(
            &target_tile,
            zoom_level,
            0..row_count as u32,
            0..row_count as u32
        ))
    }
}

#[cfg(test)]
mod parent_tests {
    use std::marker::PhantomData;

    use bevy::{
        asset::{Handle, uuid::Uuid},
        ecs::entity::{Entity, EntityIndex},
        image::Image,
    };
    use rstest::rstest;

    use crate::load_checker::{MockLoadChecker, Tile, TileDependencyTree};

    fn entity(i: u32) -> Entity {
        Entity::from_index(EntityIndex::from_raw_u32(i).unwrap())
    }

    fn create_tile(x: u32, y: u32, z: u32) -> Tile {
        Tile::new(x, y, z, entity(0))
    }

    #[rstest]
    #[case(0, 0)]
    #[case(0, 1)]
    #[case(1, 0)]
    #[case(1, 1)]
    fn parent_ready_zoom_diff_1(#[case] x: u32, #[case] y: u32) {
        let mut checker = MockLoadChecker::new();
        checker.expect_asset_is_ready().times(1).return_const(true);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(x, y, 1);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 0, 0),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        assert!(loading_tiles.parent_ready(&target_tile, 1))
    }

    #[rstest]
    #[case(0, 0)]
    #[case(0, 1)]
    #[case(1, 0)]
    #[case(1, 1)]
    fn parent_not_ready_zoom_diff_1(#[case] x: u32, #[case] y: u32) {
        let mut checker = MockLoadChecker::new();
        checker.expect_asset_is_ready().times(1).return_const(false);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(x, y, 1);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        loading_tiles.new_tile(
            &create_tile(0, 0, 0),
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        assert!(!loading_tiles.parent_ready(&target_tile, 1))
    }

    #[rstest]
    #[case(0, 0)]
    #[case(0, 1)]
    #[case(1, 0)]
    #[case(1, 1)]
    fn parent_not_spawned_zoom_diff_1(#[case] x: u32, #[case] y: u32) {
        let mut checker = MockLoadChecker::new();
        checker.expect_asset_is_ready().never();

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let target_tile = create_tile(x, y, 1);
        loading_tiles.new_tile(
            &target_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        assert!(!loading_tiles.parent_ready(&target_tile, 1))
    }

    #[rstest]
    #[case(2)]
    #[case(3)]
    #[case(4)]
    #[case(5)]
    #[case(6)]
    #[case(7)]
    #[case(8)]
    fn parent_ready_zoom_diff_greater_than_1(#[case] zoom_diff: u32) {
        let row_count = 2_usize.pow(zoom_diff);
        let count_child_tiles = row_count * row_count;
        let mut checker = MockLoadChecker::new();

        checker
            .expect_asset_is_ready()
            .times(count_child_tiles)
            .return_const(true);

        let mut loading_tiles = TileDependencyTree::new(0, checker);

        let parent_tile = create_tile(0, 0, 0);
        loading_tiles.new_tile(
            &parent_tile,
            Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
            entity(1),
        );

        for x in 0..row_count {
            for y in 0..row_count {
                let target_tile = create_tile(x as u32, y as u32, zoom_diff);
                loading_tiles.new_tile(
                    &target_tile,
                    Handle::Uuid(Uuid::new_v4(), PhantomData::<fn() -> Image> {}),
                    entity(1),
                );
                assert!(loading_tiles.parent_ready(&target_tile, zoom_diff))
            }
        }
    }
}
