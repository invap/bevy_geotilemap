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

//! In this example we show how to set up markers using the component [`MapMarker`].
//!
//! Right click to spawn a marker at cursor position.
use bevy::{
    DefaultPlugins,
    app::{App, PostUpdate, Startup},
    asset::AssetServer,
    camera::{Camera, Camera2d, OrthographicProjection, Projection, ScalingMode},
    ecs::{
        component::Component,
        entity::{ContainsEntity, Entity},
        observer::On,
        system::{Commands, Query, Res, Single},
    },
    math::Vec2,
    picking::{events::PointerClick, pointer::PointerButton},
    sprite::Sprite,
    transform::components::{GlobalTransform, Transform},
    utils::default,
};
use bevy_geotilemap::{
    GeoTilemap, GeoTilemapPlugin,
    controls::ControllableMap,
    geo_coords::{lat_lon_to_world, mts_to_tiles, world_to_lat_lon},
    marker::MapMarker,
};

#[derive(Component)]
pub struct ResizableMarker {
    pub scale: Vec2,
}

const LAT_DEG: f32 = -41.124_397;
const LON_DEG: f32 = -71.242_38;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(GeoTilemapPlugin::default())
        .add_systems(Startup, spawn_map_app)
        .add_systems(PostUpdate, rescale_relative_markers);

    app.run();
}

fn spawn_map_app(mut commands: Commands) {
    let initial_location = lat_lon_to_world(LAT_DEG, LON_DEG, GeoTilemap::default_size());
    let initial_scale = mts_to_tiles(3_000.0, LAT_DEG, GeoTilemap::default_size());

    // Spawn the 2D camera
    let camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                ..default()
            },
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

    commands
        .spawn((
            GeoTilemap {
                camera: Some(camera),
                url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
                ..default()
            },
            ControllableMap::default(),
        ))
        .observe(place_marker);
}

pub fn rescale_relative_markers(
    cameras: Query<&Projection>,
    maps: Query<&GeoTilemap>,
    mut markers: Query<(&mut Sprite, &ResizableMarker)>,
) {
    for map in maps {
        if let Some(map_camera) = map.camera
            && let Ok(projection) = cameras.get(map_camera)
            && let Projection::Orthographic(orthographic_projection) = projection
        {
            let camera_area = orthographic_projection.area;

            for (mut sprite, marker) in &mut markers {
                let camera_width = camera_area.width();

                sprite.custom_size = Some(Vec2 {
                    x: camera_width * marker.scale.x,
                    y: camera_width * marker.scale.x,
                });
            }
        }
    }
}

pub fn place_marker(
    pointer: On<PointerClick>,
    mut commands: Commands,
    geotilemap: Single<(Entity, &GeoTilemap)>,
    cameras: Query<(&GlobalTransform, &Camera)>,
    asset_server: Res<AssetServer>,
) {
    if let Some(camera_entity) = geotilemap.1.camera
        && let Ok((global_transform, camera)) = cameras.get(camera_entity)
        && let Ok(location) =
            camera.viewport_to_world_2d(global_transform, pointer.pointer.location().position)
        && pointer.button == PointerButton::Secondary
    {
        let degrees = world_to_lat_lon(location, GeoTilemap::default_size());

        commands.entity(geotilemap.0.entity()).with_child((
            Sprite {
                image: asset_server.load("pinpoint_red_fill.png"),
                ..default()
            },
            MapMarker {
                latitude: degrees.x,
                longitude: degrees.y,
            },
            Transform::from_xyz(0.0, 0.0, 1.0),
            ResizableMarker {
                scale: Vec2::new(0.2, 0.2),
            },
        ));
    }
}
