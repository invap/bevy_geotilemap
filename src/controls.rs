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

use std::time::Duration;

use bevy::{
    app::{Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        observer::On,
        query::Added,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query},
    },
    input::mouse::MouseScrollUnit,
    math::{NormedVectorSpace, Vec2, Vec3},
    picking::{
        events::{PointerDrag, PointerRelease, PointerScroll},
        pointer::PointerButton,
    },
    time::common_conditions::on_real_timer,
    utils::default,
};

use crate::{GeoTilemap, camera_properties::CameraProperties};

pub struct MapControlsPlugin;

#[derive(Component, Clone, Debug)]
pub struct ControllableMap {
    locked: bool,
    velocity: Vec2,
    /// Tipical values between 0 and 1, values out of this range
    /// produce erratic behaviour
    pub friction: f32,
}

impl Default for ControllableMap {
    fn default() -> Self {
        Self {
            locked: false,
            velocity: Vec2 { x: 0.0, y: 0.0 },
            friction: 0.07,
        }
    }
}

impl ControllableMap {
    pub fn new(friction: f32) -> Self {
        Self {
            friction,
            ..default()
        }
    }
}

impl Plugin for MapControlsPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            Update,
            (
                observe_controllable_maps,
                slide_camera_view.run_if(on_real_timer(Duration::from_millis(10))),
            ),
        );
    }
}

fn observe_controllable_maps(
    mut commands: Commands,
    controllable_maps: Query<Entity, Added<GeoTilemap>>,
) {
    for entity in controllable_maps {
        commands
            .entity(entity)
            .observe(move_camera_view)
            .observe(release_camera_view)
            .observe(zoom_map_camera);
    }
}

/// On drag move camera
/// converting from relative image drag to world drag
fn move_camera_view(
    pointer: On<PointerDrag>,
    mut geotilemaps: Query<(&GeoTilemap, &mut ControllableMap)>,
    mut camera_properties: CameraProperties,
) {
    if let Ok((geotilemap, mut controllable_map)) = geotilemaps.get_mut(pointer.entity)
        && let Some(camera_entity) = geotilemap.camera
        && let Some((orthographic_projection, transform, _, camera)) =
            camera_properties.get_mut_camera_transforms(camera_entity)
        && let Some(physical_size) = camera.physical_viewport_size()
        && pointer.button == PointerButton::Primary
    {
        controllable_map.locked = true;

        if (pointer.delta / physical_size.as_vec2()).norm() > 0.009 {
            controllable_map.velocity = pointer.delta;
        } else {
            controllable_map.velocity = Vec2 { x: 0.0, y: 0.0 };
        }

        // Convert drag to camera movement
        let translate_x =
            -(pointer.delta.x / physical_size.x as f32 * orthographic_projection.area.width());

        let translate_y =
            pointer.delta.y / physical_size.y as f32 * orthographic_projection.area.height();

        transform.translation += Vec3::new(translate_x, translate_y, 0.0);
    }
}

/// When a [`ControllableMap`] is released change its locked state to false to begin free movement
fn release_camera_view(pointer: On<PointerRelease>, mut geotilemaps: Query<&mut ControllableMap>) {
    if let Ok(mut controllable_map) = geotilemaps.get_mut(pointer.entity) {
        controllable_map.locked = false;
    }
}

/// Slide [`ControllableMap`] camera and decrease its velocity until stopped
fn slide_camera_view(
    geotilemaps: Query<(&GeoTilemap, &mut ControllableMap)>,
    mut camera_properties: CameraProperties,
) {
    for (geotilemap, mut controllable_map) in geotilemaps {
        if !controllable_map.locked
            && controllable_map.velocity.norm() > 1e-6
            && let Some(camera_entity) = geotilemap.camera
            && let Some((orthographic_projection, transform, _, camera)) =
                camera_properties.get_mut_camera_transforms(camera_entity)
            && let Some(physical_size) = camera.physical_viewport_size()
        {
            let translate_x = -(controllable_map.velocity.x / physical_size.x as f32
                * orthographic_projection.area.width());

            let translate_y = controllable_map.velocity.y / physical_size.y as f32
                * orthographic_projection.area.height();

            transform.translation += Vec3::new(translate_x, translate_y, 0.0);

            let friction = controllable_map.friction;
            controllable_map.velocity *= 1.0 - friction;
        }
    }
}

/// Handle camera zoom using scroll when hovering over a map
fn zoom_map_camera(
    scroll: On<PointerScroll>,
    mut geotilemaps: Query<(&GeoTilemap, &mut ControllableMap)>,
    mut camera_properties: CameraProperties,
) {
    if let Ok((geotilemap, mut controllable_map)) = geotilemaps.get_mut(scroll.entity)
        && let Some(camera_entity) = geotilemap.camera
        && let Some((orthographic_projection, transform, global_transform, camera)) =
            camera_properties.get_mut_camera_transforms(camera_entity)
    {
        controllable_map.velocity = Vec2 { x: 0.0, y: 0.0 };

        let pointer_location = &scroll.pointer.location();

        // Compute pointer position pre zoom
        let res_pointer_world_coords_pre_zoom =
            camera.viewport_to_world_2d(global_transform, pointer_location.position);

        let delta = match scroll.unit {
            MouseScrollUnit::Line => 0.2,
            MouseScrollUnit::Pixel => 0.0025,
        };

        // Zoom projection
        let delta_zoom = -scroll.y * delta + 1.0;
        orthographic_projection.scale *= delta_zoom;

        if let Ok(pointer_world_coords_pre_zoom) = res_pointer_world_coords_pre_zoom {
            let cam_pos = global_transform.translation().truncate();

            // Compute pointer position post zoom
            let pointer_world_coords_post_zoom =
                cam_pos + (pointer_world_coords_pre_zoom - cam_pos) * delta_zoom;

            let difference = pointer_world_coords_pre_zoom - pointer_world_coords_post_zoom;

            transform.translation += difference.extend(0.0);
        }
    }
}
