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
    camera::{Camera, OrthographicProjection, Projection},
    ecs::{
        entity::Entity,
        query::Without,
        system::{Query, SystemParam},
    },
    math::{Mat4, UVec2, Vec2, Vec4},
    transform::components::{GlobalTransform, Transform},
};

use crate::{GeoTilemap, geo_coords::to_tile_coord};

#[derive(SystemParam)]
pub struct CameraProperties<'w, 's> {
    cameras: Query<
        'w,
        's,
        (
            &'static mut Projection,
            &'static mut Transform,
            &'static GlobalTransform,
            &'static Camera,
        ),
        Without<GeoTilemap>,
    >,
}

impl CameraProperties<'_, '_> {
    pub fn get_camera_projection_and_transform(
        &self,
        camera_entity: Entity,
    ) -> Option<(OrthographicProjection, Transform)> {
        let (projection, transform, _, _) = self.cameras.get(camera_entity).ok()?;

        let Projection::Orthographic(orthographic_projection) = projection else {
            return None;
        };

        Some((orthographic_projection.clone(), *transform))
    }

    #[cfg(feature = "controls")]
    pub fn get_mut_camera_transforms(
        &mut self,
        camera_entity: Entity,
    ) -> Option<(
        &mut OrthographicProjection,
        &mut Transform,
        &GlobalTransform,
        &Camera,
    )> {
        let (projection, transform, global_transform, camera) =
            self.cameras.get_mut(camera_entity).ok()?;

        match projection.into_inner() {
            Projection::Orthographic(orthographic_projection) => Some((
                orthographic_projection,
                transform.into_inner(),
                global_transform,
                camera,
            )),
            _ => None,
        }
    }

    pub fn get_camera_tile_ranges(
        &self,
        camera_entity: Entity,
        geotilemap: &GeoTilemap,
        map_transform: &Transform,
    ) -> Option<(Range<u32>, Range<u32>)> {
        let (orthographic_projection, camera_transform) =
            self.get_camera_projection_and_transform(camera_entity)?;

        let tile_count = 2_u32.pow(geotilemap.zoom_level);

        let sprite_size =
            geotilemap.custom_size.unwrap_or(GeoTilemap::default_size()) / tile_count as f32;

        // Get current ranges
        Some(Self::compute_rect_ranges(
            &camera_transform,
            tile_count,
            sprite_size,
            &orthographic_projection,
            map_transform,
            geotilemap.loading_area_scale,
        ))
    }

    /// Compute loading ranges using camera location and view
    fn compute_rect_ranges(
        camera_transform: &Transform,
        tile_count: u32,
        sprite_size: Vec2,
        orthographic_projection: &OrthographicProjection,
        map_transform: &Transform,
        loading_area_scale: f32,
    ) -> (Range<u32>, Range<u32>) {
        let (min, max) = Self::camera_view_to_tile_ranges(
            orthographic_projection,
            camera_transform,
            map_transform,
            sprite_size,
            tile_count,
            loading_area_scale,
        );

        (min.x..max.x + 1, max.y..min.y + 1)
    }

    /// Transform camera view to slippy tile coordinates, taking into account map transform
    pub fn camera_view_to_tile_ranges(
        camera_projection: &OrthographicProjection,
        camera_transform: &Transform,
        map_transform: &Transform,
        sprite_size: Vec2,
        tile_count: u32,
        loading_area_scale: f32,
    ) -> (UVec2, UVec2) {
        // We compute the inverse of the map transform to obtain camera ranges in new basis
        let inv_map_transform = map_transform.to_matrix().inverse();

        let camera_viewport_vertices =
            Self::camera_vertex_matrix(camera_projection, camera_transform, loading_area_scale);

        // Transform each camera vertex to untransformed map basis
        let transformed_camera_rect = inv_map_transform * camera_viewport_vertices;

        let x_map_view_min = transformed_camera_rect.row(0).min_element();
        let x_map_view_max = transformed_camera_rect.row(0).max_element();
        let y_map_view_min = transformed_camera_rect.row(1).min_element();
        let y_map_view_max = transformed_camera_rect.row(1).max_element();

        let min = to_tile_coord(
            Vec2::new(x_map_view_min, y_map_view_min),
            sprite_size,
            tile_count as i32,
        );
        let max = to_tile_coord(
            Vec2::new(x_map_view_max, y_map_view_max),
            sprite_size,
            tile_count as i32,
        );

        (min, max)
    }

    /// Compute a matrix in which columns are camera viewport rect vertices in homogeneous world coordinates
    fn camera_vertex_matrix(
        camera_projection: &OrthographicProjection,
        camera_transform: &Transform,
        loading_area_scale: f32,
    ) -> Mat4 {
        // Upper right corner of camera viewport in world coords
        let upper_right_vertex = Vec2::new(
            camera_transform.translation.x
                + loading_area_scale * camera_projection.area.width() / 2.0,
            camera_transform.translation.y
                + loading_area_scale * camera_projection.area.height() / 2.0,
        );

        // Lower left corner of camera viewport in world coords
        let lower_left_vertex = Vec2::new(
            camera_transform.translation.x
                - loading_area_scale * camera_projection.area.width() / 2.0,
            camera_transform.translation.y
                - loading_area_scale * camera_projection.area.height() / 2.0,
        );

        Mat4::from_cols(
            Vec4::new(lower_left_vertex.x, lower_left_vertex.y, 0.0, 1.0),
            Vec4::new(lower_left_vertex.x, upper_right_vertex.y, 0.0, 1.0),
            Vec4::new(upper_right_vertex.x, upper_right_vertex.y, 0.0, 1.0),
            Vec4::new(upper_right_vertex.x, lower_left_vertex.y, 0.0, 1.0),
        )
    }
}
