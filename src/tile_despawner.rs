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

use std::ops::Range;

use bevy::{
    app::{App, Plugin, Update},
    asset::AssetServer,
    ecs::system::{Commands, Query, Res, ResMut},
    log::error,
    transform::components::Transform,
};

use crate::{
    GeoTilemap,
    camera_properties::CameraProperties,
    load_checker::{Tile, TileAsset, TileDependencyTree, TileLoadCheck},
    pipeline_ready::{PipelinesReady, PipelinesReadyPlugin},
};

#[derive(Default)]
pub struct TileDespawner {
    pub confirmation_frames: usize,
}

impl TileDespawner {
    pub fn new(confirmation_frames: usize) -> Self {
        Self {
            confirmation_frames,
        }
    }
}

impl Plugin for TileDespawner {
    fn build(&self, app: &mut App) {
        if let Some(asset_server) = app.world().get_resource::<AssetServer>() {
            app.insert_resource(TileDependencyTree::new(
                self.confirmation_frames,
                TileLoadCheck {
                    asset_server: asset_server.clone(),
                },
            ))
            .add_plugins(PipelinesReadyPlugin)
            .add_systems(Update, despawn_tiles_out_of_range);
        } else {
            error!("Asset server resource not found");
        }
    }
}

pub fn despawn_tiles_out_of_range(
    mut commands: Commands,
    mut loading_tiles: ResMut<TileDependencyTree<TileLoadCheck>>,
    pipelines_ready: Res<PipelinesReady>,
    camera_properties: CameraProperties,
    geotilemaps: Query<(&GeoTilemap, &Transform)>,
) {
    let loading_tiles_clone = loading_tiles.clone();

    loading_tiles.handle_hash.retain(|tile, tile_asset| {
        if let Ok((geotilemap, geotilemap_transform)) = geotilemaps.get(tile.geotilemap)
            && let Some(camera) = geotilemap.camera
            && let Some((x_range, y_range)) =
                camera_properties.get_camera_tile_ranges(camera, geotilemap, geotilemap_transform)
        {
            // Same zoom level and out of extended camera rect
            if tile.z == geotilemap.zoom_level {
                handle_tile_out_of_view(&mut commands, tile, tile_asset, &x_range, &y_range)
            } else if geotilemap.zoom_level > tile.z {
                handle_zoom_in(
                    &mut commands,
                    &pipelines_ready,
                    &loading_tiles_clone,
                    tile,
                    tile_asset,
                    x_range,
                    y_range,
                    geotilemap,
                )
            } else {
                handle_zoom_out(
                    &mut commands,
                    &pipelines_ready,
                    &loading_tiles_clone,
                    tile,
                    tile_asset,
                    geotilemap,
                )
            }
        } else {
            true
        }
    });
}

fn handle_zoom_out(
    commands: &mut Commands,
    pipelines_ready: &Res<PipelinesReady>,
    loading_tiles_clone: &TileDependencyTree<TileLoadCheck>,
    tile: &Tile,
    tile_asset: &mut TileAsset,
    geotilemap: &GeoTilemap,
) -> bool {
    if pipelines_ready.0 && loading_tiles_clone.parent_ready(tile, tile.z - geotilemap.zoom_level) {
        handle_frame_confirmation(commands, loading_tiles_clone, tile_asset)
    } else {
        true
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_zoom_in(
    commands: &mut Commands,
    pipelines_ready: &Res<PipelinesReady>,
    loading_tiles_clone: &TileDependencyTree<TileLoadCheck>,
    tile: &Tile,
    tile_asset: &mut TileAsset,
    x_range: Range<u32>,
    y_range: Range<u32>,
    geotilemap: &GeoTilemap,
) -> bool {
    if pipelines_ready.0
        && loading_tiles_clone.children_ready(
            tile,
            geotilemap.zoom_level - tile.z,
            x_range,
            y_range,
        )
    {
        handle_frame_confirmation(commands, loading_tiles_clone, tile_asset)
    } else {
        true
    }
}

fn handle_frame_confirmation(
    commands: &mut Commands,
    loading_tiles_clone: &TileDependencyTree<TileLoadCheck>,
    tile_asset: &mut TileAsset,
) -> bool {
    if tile_asset.ready_to_despawn_countdown == loading_tiles_clone.confirmation_frames {
        commands.entity(tile_asset.target_entity).despawn();
        false
    } else {
        tile_asset.ready_to_despawn_countdown += 1;
        true
    }
}

fn handle_tile_out_of_view(
    commands: &mut Commands,
    tile: &Tile,
    tile_asset: &mut TileAsset,
    x_range: &Range<u32>,
    y_range: &Range<u32>,
) -> bool {
    if !x_range.contains(&tile.x) || !y_range.contains(&tile.y) {
        commands.entity(tile_asset.target_entity).despawn();
        false
    } else {
        true
    }
}
