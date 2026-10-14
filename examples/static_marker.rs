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

//! In this example we show how to create markers using the component [`MapMarker`].
use bevy::{
    DefaultPlugins,
    app::{App, Startup},
    asset::AssetServer,
    camera::{Camera, Camera2d, OrthographicProjection, Projection, ScalingMode},
    ecs::system::{Commands, Res},
    math::Vec2,
    sprite::Sprite,
    transform::components::Transform,
    utils::default,
};
use bevy_geotilemap::{
    GeoTilemap, GeoTilemapPlugin,
    controls::ControllableMap,
    geo_coords::{lat_lon_to_world, mts_to_tiles},
    marker::MapMarker,
};

const LAT_DEG: f32 = -41.124_397;
const LON_DEG: f32 = -71.242_38;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(GeoTilemapPlugin::default())
        .add_systems(Startup, spawn_map_app);

    app.run();
}

fn spawn_map_app(mut commands: Commands, asset_server: Res<AssetServer>) {
    let initial_location = lat_lon_to_world(LAT_DEG, LON_DEG, GeoTilemap::default_size());
    let initial_scale = mts_to_tiles(3_000.0, LAT_DEG, GeoTilemap::default_size());

    let camera = commands
        .spawn((
            Camera2d,
            Camera { ..default() },
            Transform::from_xyz(initial_location.x, initial_location.y, -1.0),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::AutoMax {
                    max_width: 1.0,
                    max_height: 1.0,
                },
                // Initial scale corresponds to the extension of land seen by the camera
                // based on the perimeter of the earth on the equator
                scale: initial_scale,
                ..OrthographicProjection::default_2d()
            }),
        ))
        .id();

    // Create marker of side length of 5km. Notice that on mercator
    // projection, meters depend on latitude due to deformation of lengths
    let marker_size = mts_to_tiles(100.0, LAT_DEG, GeoTilemap::default_size());

    commands
        .spawn((
            GeoTilemap {
                camera: Some(camera),
                url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
                ..default()
            },
            ControllableMap::default(),
        ))
        .with_child((
            Sprite {
                image: asset_server.load("pinpoint_red_fill.png"),
                custom_size: Some(Vec2::new(marker_size, marker_size)),
                ..default()
            },
            MapMarker {
                latitude: LAT_DEG + 1e-3,
                longitude: LON_DEG + 1e-3,
            },
            Transform::from_xyz(0.0, 0.0, 1.0),
        ));
}
