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

use bevy::{
    app::{Plugin, PostUpdate},
    camera::Projection,
    ecs::{
        component::Component,
        hierarchy::ChildOf,
        query::Changed,
        schedule::{IntoScheduleConfigs, SystemSet},
        system::Query,
    },
    math::{Vec2, Vec3},
    transform::components::Transform,
};

use crate::{GeoTilemap, geo_coords::lat_lon_to_world};

pub struct MapMarkerPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MarkerPluginSet;

impl Plugin for MapMarkerPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            PostUpdate,
            (update_map_markers, rescale_relative_markers).chain(),
        );
    }
}

/// Marker with an absolute size in world
/// # Examples
///
/// Spawn a circular marker with a radius of 10km
///
/// ```ignore
/// let latitude = -41.18;
///
/// commands.spawn((
///    Transform::from_xyz(0.0, 0.0, 1.0),
///    RenderLayers::layer(1),
///    Mesh2d(meshes.add(Circle::new(mts_to_tiles(10_000.0, latitude)))),
///    MapMarker {
///        latitude: latitude,
///        longitude: 0.0,
///    },
///    MeshMaterial2d(materials.add(Color::srgba_u8(128, 0, 0, 80))),
/// ));
/// ```
#[derive(Component, Clone, Debug)]
#[require(Transform)]
pub struct MapMarker {
    /// Latitude in degrees
    pub latitude: f32,
    /// Longitude in degrees
    pub longitude: f32,
}

impl Default for MapMarker {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}

/// Update [`MapMarker`] position
fn update_map_markers(
    maps: Query<&GeoTilemap>,
    markers: Query<(&ChildOf, &mut Transform, &MapMarker), Changed<MapMarker>>,
) {
    for (childof, mut transform, marker) in markers {
        if let Ok(geotilemap) = maps.get(childof.parent()) {
            let coord = lat_lon_to_world(
                marker.latitude,
                marker.longitude,
                geotilemap.custom_size.unwrap_or(GeoTilemap::default_size()),
            );

            transform.translation = Vec3::new(coord.x, coord.y, transform.translation.z);
        }
    }
}

/// Marker with an absolute size in world
/// # Examples
///
/// Spawn a circular marker with a radius that scales relative to camera area size
///
/// ```ignore
/// let latitude = -41.18;
///
/// commands.spawn((
///    Transform::from_xyz(0.0, 0.0, 1.0),
///    RenderLayers::layer(1),
///    Mesh2d(meshes.add(Circle::new(1.0))),
///    MapMarker {
///        latitude: latitude,
///        longitude: 0.0,
///    },
///    MeshMaterial2d(materials.add(Color::srgba_u8(128, 0, 0, 80))),
///    RelativeMarker {
///        scale: Vec2::new(0.2, 0.2),
///        ..default()
///    }
/// ));
/// ```
#[derive(Component, Clone, Debug)]
pub struct RelativeMarker {
    pub scale: Vec2,
    pub min_size: Vec2,
    pub max_size: Vec2,
}

impl Default for RelativeMarker {
    fn default() -> Self {
        Self {
            scale: Vec2::ONE,
            min_size: Vec2::ZERO,
            max_size: Vec2::INFINITY,
        }
    }
}

fn rescale_relative_markers(
    cameras: Query<&Projection>,
    maps: Query<&GeoTilemap>,
    mut markers: Query<(&mut Transform, &RelativeMarker)>,
) {
    for map in maps {
        if let Some(map_camera) = map.camera
            && let Ok(projection) = cameras.get(map_camera)
            && let Projection::Orthographic(orthographic_projection) = projection
        {
            let camera_area = orthographic_projection.area;

            for (mut transform, marker) in &mut markers {
                let camera_width = camera_area.width();

                transform.scale = Vec3::new(
                    (camera_width * marker.scale.x).clamp(marker.min_size.x, marker.max_size.x),
                    (camera_width * marker.scale.y).clamp(marker.min_size.y, marker.max_size.y),
                    1.0,
                );
            }
        }
    }
}
