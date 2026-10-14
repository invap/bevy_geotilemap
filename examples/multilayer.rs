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

//! In this example we show how to have multiple layers, for instance to see street and political data over satellite layer
use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup, Startup},
    asset::{AssetMetaCheck, AssetPlugin},
    camera::{
        Camera, Camera2d, OrthographicProjection, Projection, ScalingMode, visibility::RenderLayers,
    },
    ecs::system::Commands,
    transform::components::Transform,
    utils::default,
};
use bevy_geotilemap::{
    GeoTilemap, GeoTilemapPlugin,
    controls::ControllableMap,
    geo_coords::{lat_lon_to_world, mts_to_tiles},
};

const LAT_DEG: f32 = -41.124_397;
const LON_DEG: f32 = -71.242_38;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(AssetPlugin {
        meta_check: AssetMetaCheck::Never,
        ..default()
    }))
    .add_plugins(GeoTilemapPlugin::default())
    .add_systems(Startup, spawn_map_app);

    app.run();
}

fn spawn_map_app(mut commands: Commands) {
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
            RenderLayers::layer(1),
        ))
        .id();

    commands.spawn((
        GeoTilemap {
            camera: Some(camera),
            url: "https://wms.ign.gob.ar/geoserver/gwc/service/tms/1.0.0/mapabase_hibrido@EPSG%3A3857@png/{z}/{x}/{-y}.png".to_string(),
            ..default()
        },
        ControllableMap::default(),
        Transform::from_xyz(0.0, 0.0, 1.0),
        RenderLayers::layer(1),
    ));

    commands.spawn((
        GeoTilemap {
            camera: Some(camera),
            url: "https://server.arcgisonline.com/arcgis/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}.jpeg".to_string(),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        ControllableMap::default(),
        RenderLayers::layer(1),
    ));
}
