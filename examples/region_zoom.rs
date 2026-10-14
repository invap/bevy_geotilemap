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

//! In this example we show how to create more complicated controlls like region zoom.
//!
//! Press the scroll wheel and drag to zoom into a rectangular region.
use bevy::{
    DefaultPlugins,
    app::{App, Startup},
    asset::asset_value,
    camera::{
        Camera, Camera2d, OrthographicProjection, Projection, ScalingMode,
        visibility::{RenderLayers, Visibility},
    },
    color::palettes::css::RED,
    ecs::{
        component::Component,
        entity::{ContainsEntity, Entity},
        hierarchy::Children,
        observer::On,
        query::With,
        resource::Resource,
        system::{Commands, Res, ResMut, Single},
    },
    input::{ButtonInput, keyboard::KeyCode},
    math::Vec2,
    mesh::Mesh2d,
    picking::{
        Pickable,
        events::{PointerLeave, PointerMove, PointerPress, PointerRelease},
        pointer::PointerButton,
    },
    scene::{CommandsSceneExt, bsn},
    shape::Polyline2d,
    sprite_render::{ColorMaterial, MeshMaterial2d},
    transform::components::{GlobalTransform, Transform},
    utils::default,
};
use bevy_geotilemap::{
    GeoTilemap, GeoTilemapPlugin,
    controls::ControllableMap,
    geo_coords::{lat_lon_to_world, mts_to_tiles},
};

const LAT_DEG: f32 = -41.124_397;
const LON_DEG: f32 = -71.242_38;

#[derive(Resource)]
pub enum SelectedRegion {
    None,
    Some {
        start_coords: Vec2,
        end_coords: Vec2,
    },
}

#[derive(Component, Debug, Default, Clone)]
pub struct ZoomRegion;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(GeoTilemapPlugin::default())
        .insert_resource(SelectedRegion::None)
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

    commands
        .spawn((
            GeoTilemap {
                camera: Some(camera),
                url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
                ..default()
            },
            ControllableMap::default(),
            Pickable::default(),
            RenderLayers::layer(1),
        ))
        .observe(initialize_zoom_region)
        .observe(update_zoom_region)
        .observe(trigger_zoom_release)
        .observe(trigger_zoom_drag_leave);
}

fn initialize_zoom_region(
    key: On<PointerPress>,
    mut commands: Commands,
    mut zoom_region: ResMut<SelectedRegion>,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    if PointerButton::Middle == key.button
        && let Ok(cursor_position) = camera
            .0
            .viewport_to_world_2d(camera.1, key.pointer.location().position)
    {
        *zoom_region = SelectedRegion::Some {
            start_coords: cursor_position,
            end_coords: cursor_position,
        };

        commands.spawn_scene(bsn! {
            ZoomRegion
            Transform::from_translation(cursor_position.extend(1.0))
            Visibility::Hidden
            Children [
                RenderLayers::layer(1)
                Transform::from_xyz(0.0, 0.0, 0.0)
                Mesh2d(asset_value(Polyline2d::new(vec![
                            Vec2::new(0.0, 0.0),
                            Vec2::new(1.0, 0.0),
                            Vec2::new(1.0, 1.0),
                            Vec2::new(0.0, 1.0),
                            Vec2::new(0.0, 0.0),])))
                MeshMaterial2d<ColorMaterial>(asset_value(ColorMaterial::from_color(RED)))
                Visibility
            ]
        });
    }
}

fn update_zoom_region(
    key: On<PointerMove>,
    mut zoom_region: ResMut<SelectedRegion>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut rectangle: Single<(&mut Visibility, &mut Transform), With<ZoomRegion>>,
) {
    if let Ok(cursor_position) = camera
        .0
        .viewport_to_world_2d(camera.1, key.pointer.location().position)
        && let SelectedRegion::Some {
            start_coords,
            end_coords,
        } = zoom_region.as_mut()
    {
        *end_coords = cursor_position;
        rectangle.1.scale = (cursor_position - *start_coords).extend(1.0);
        *rectangle.0 = Visibility::Inherited;
    }
}

fn trigger_zoom_release(
    key: On<PointerRelease>,
    ctrl: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    zoom_region: ResMut<SelectedRegion>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera>>,
    rectangle: Single<Entity, With<ZoomRegion>>,
) {
    if PointerButton::Middle == key.button {
        trigger_zoom(ctrl, &mut commands, zoom_region, camera, rectangle);
    }
}

pub fn trigger_zoom_drag_leave(
    _: On<PointerLeave>,
    ctrl: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    zoom_region: ResMut<SelectedRegion>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera>>,
    rectangle: Single<Entity, With<ZoomRegion>>,
) {
    trigger_zoom(ctrl, &mut commands, zoom_region, camera, rectangle);
}

fn trigger_zoom(
    ctrl: Res<ButtonInput<KeyCode>>,
    commands: &mut Commands,
    zoom_region: ResMut<SelectedRegion>,
    camera: Single<(&mut Transform, &mut Projection), With<Camera>>,
    rectangle: Single<Entity, With<ZoomRegion>>,
) {
    let (mut transform, mut proj) = camera.into_inner();
    let SelectedRegion::Some {
        start_coords,
        end_coords,
    } = *zoom_region
    else {
        return;
    };
    let Projection::Orthographic(orthographic_proj) = proj.as_mut() else {
        return;
    };
    let max_diff = (end_coords - start_coords).abs().max_element();
    let camera_max_side = Vec2::new(
        orthographic_proj.area.width(),
        orthographic_proj.area.height(),
    )
    .max_element();

    if max_diff > 1e-6 {
        let mid_point = start_coords + (end_coords - start_coords) / 2.0;

        transform.translation = mid_point.extend(0.0);

        let scale = if ctrl.pressed(KeyCode::ControlLeft) || ctrl.pressed(KeyCode::ControlRight) {
            camera_max_side / max_diff
        } else {
            max_diff / camera_max_side
        };

        orthographic_proj.scale *= scale;
    };
    commands.insert_resource(SelectedRegion::None);
    commands.entity(rectangle.entity()).despawn();
}
