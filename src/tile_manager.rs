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

//! Tile spawn and despawn management module
use std::ops::Range;

use bevy::{
    ecs::{
        entity::Entity,
        event::Event,
        observer::On,
        query::With,
        system::{Commands, Query, ResMut},
    },
    math::UVec2,
    picking::Pickable,
    sprite::Sprite,
    transform::components::Transform,
    utils::default,
};

use crate::{
    GeoTilemap, RedrawMaps,
    camera_properties::CameraProperties,
    geo_coords::to_world_coord,
    load_checker::{Tile, TileDependencyTree, TileLoadCheck},
    tile_cache::TileCache,
};

/// Spawn tile rectangle area event
#[derive(Event, Debug)]
pub struct SpawnTileMapRect {
    /// X axis range
    pub x_range: Range<u32>,
    /// Y axis range
    pub y_range: Range<u32>,
    /// GeoTileMap
    pub geotilemap: GeoTilemap,
    /// GeotileMap entity
    pub geotilemap_entity: Entity,
}

/// Trigger a [`SpawnTileMapRect`] event for [`GeoTilemap`]
/// using camera view and location
pub fn load_tile_map_in_view(
    camera_properties: CameraProperties,
    geotilemaps: Query<(Entity, &GeoTilemap, &Transform)>,
    mut commands: Commands,
) {
    for (geotilemap_entity, geotilemap, geotilemap_transform) in geotilemaps {
        if let Some(camera) = geotilemap.camera
            && let Some((x_range, y_range)) =
                camera_properties.get_camera_tile_ranges(camera, geotilemap, geotilemap_transform)
        {
            commands.trigger(SpawnTileMapRect {
                x_range,
                y_range,
                geotilemap: geotilemap.clone(),
                geotilemap_entity,
            });
        }
    }
}

/// Spawn a tile map rectangle area when a [`SpawnTileMapRect`] event
/// triggers
pub fn spawn_tile_map_rect(
    event: On<SpawnTileMapRect>,
    mut commands: Commands,
    mut tile_cache: ResMut<TileCache>,
    mut loading_tiles: ResMut<TileDependencyTree<TileLoadCheck>>,
) {
    for x in event.x_range.clone() {
        for y in event.y_range.clone() {
            let current_tile = Tile {
                x,
                y,
                z: event.geotilemap.zoom_level,
                geotilemap: event.geotilemap_entity,
            };

            // Check if requested tile is already spawned
            if !loading_tiles.handle_hash.contains_key(&current_tile) {
                let tile_count = 2_u32.pow(event.geotilemap.zoom_level);
                let sprite_size = event
                    .geotilemap
                    .custom_size
                    .unwrap_or(GeoTilemap::default_size())
                    / tile_count as f32;

                let sprite_coord = to_world_coord(UVec2::new(x, y), sprite_size, tile_count as i32);

                let tile_image =
                    tile_cache.load(x, y, event.geotilemap.zoom_level, &event.geotilemap.url);

                let tile_entity = commands
                    .spawn((
                        Sprite {
                            image: tile_image.clone(),
                            custom_size: Some(sprite_size),
                            ..default()
                        },
                        Transform::from_xyz(
                            sprite_coord.x,
                            sprite_coord.y,
                            0.0 + event.geotilemap.zoom_level as f32 / 1e6,
                        ),
                        Pickable::default(),
                        current_tile,
                    ))
                    .id();

                // Spawn tile to target camera
                commands
                    .entity(event.geotilemap_entity)
                    .add_child(tile_entity);

                loading_tiles.new_tile(
                    &Tile::new(x, y, event.geotilemap.zoom_level, event.geotilemap_entity),
                    tile_image,
                    tile_entity,
                );
            }
        }
    }
}

pub fn redraw_event(
    _: On<RedrawMaps>,
    mut tile_cache: ResMut<TileCache>,
    mut commands: Commands,
    tiles: Query<Entity, With<Tile>>,
    mut loading_tiles: ResMut<TileDependencyTree<TileLoadCheck>>,
) {
    tile_cache.clear();
    loading_tiles.handle_hash.clear();

    for tile in tiles {
        commands.entity(tile).despawn();
    }
}

/// Update camera zoom level using number of tiles in view
pub fn update_zoom_level(geotilemaps: Query<&mut GeoTilemap>, camera_properties: CameraProperties) {
    for mut geotilemap in geotilemaps {
        if let Some(camera) = geotilemap.camera
            && let Some((orthographic_projection, _)) =
                camera_properties.get_camera_projection_and_transform(camera)
        {
            let height = orthographic_projection.area.height();
            let width = orthographic_projection.area.width();

            let tile_count = 2.0_f32.powf(geotilemap.zoom_level as f32);

            let sprite_size =
                geotilemap.custom_size.unwrap_or(GeoTilemap::default_size()) / tile_count;

            let min_sprite_side = if width < height {
                sprite_size.x
            } else {
                sprite_size.y
            };

            let dim = width.min(height);

            let sprites_in_view = dim / min_sprite_side;

            let zoom_out_offset = (sprites_in_view / geotilemap.zoom_policy.max_tiles_in_axis)
                .log2()
                .ceil() as u32;

            let zoom_in_offset = (geotilemap.zoom_policy.min_tiles_in_axis / sprites_in_view)
                .log2()
                .ceil() as u32;

            geotilemap.zoom_level = geotilemap
                .zoom_level
                .saturating_sub(zoom_out_offset)
                .max(geotilemap.min_zoom_level);

            geotilemap.zoom_level =
                (geotilemap.zoom_level + zoom_in_offset).min(geotilemap.max_zoom_level);
        }
    }
}
