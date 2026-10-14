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

use std::f32::consts::PI;

use bevy::math::{UVec2, Vec2};

/// Transform coordinates from slippy tiles to world
pub fn to_world_coord(tile_coord: UVec2, sprite_size: Vec2, tile_count: i32) -> Vec2 {
    Vec2::new(
        // we add sprite_size / 2.0 to accomodate for the sprite center
        sprite_size.x * (tile_coord.x as f32 + 0.5),
        sprite_size.y * (((tile_count - 1) - tile_coord.y as i32) as f32 + 0.5),
    )
}

/// Convert lat/lon to 0-1 world coordinates
pub fn lat_lon_to_world(lat_deg: f32, lon_deg: f32, map_size: Vec2) -> Vec2 {
    let x_tile = (lon_deg + 180.0) / 360.0;

    let lat_rad = lat_deg.to_radians();
    let y_tile = 1.0 - (1.0 - (lat_rad.tan() + (1.0 / lat_rad.cos())).ln() / PI) / 2.0;

    Vec2::new(x_tile * map_size.x, y_tile * map_size.y)
}

/// Convert from 0-1 world coordinates to lat/lon
pub fn world_to_lat_lon(world: Vec2, map_size: Vec2) -> Vec2 {
    let world = world / map_size;
    let lon_deg = world.x * 360.0 - 180.0;

    let y = world.y;
    let lat_rad = (std::f32::consts::PI * (1.0 - 2.0 * y)).sinh().atan();
    let lat_deg = -lat_rad.to_degrees();

    Vec2::new(lat_deg, lon_deg)
}

/// Transform coordinates from world to slippy tiles
pub fn to_tile_coord(coord: Vec2, tile_size: Vec2, tile_count: i32) -> UVec2 {
    // we divide by tile size and floor the result to get the tile number in which the requested coordinate is inside
    // then we clamp in range 0 to (tile_count - 1)
    let mut tile_coord = (coord / tile_size).floor().as_uvec2().clamp(
        UVec2 { x: 0, y: 0 },
        UVec2 {
            x: tile_count as u32 - 1,
            y: tile_count as u32 - 1,
        },
    );

    // we need to invert the value of y, because the y axis in slippy tiles is flipped to that of cartesian 2d world
    // notice that if coord was greater than map_limit, then tile_coord.y = tile_count
    tile_coord.y = tile_count as u32 - 1 - tile_coord.y;

    tile_coord
}

/// Helper function to convert from meters to absolute width in world
/// at a specific latitude.
///
/// Latitude is in degrees
pub fn mts_to_tiles(d: f32, lat: f32, map_size: Vec2) -> f32 {
    /// Perimeter of the earth at the Equator in meters
    const EARTH_PERIMETER: f32 = 40075016.68;

    (d / (lat.to_radians().cos() * EARTH_PERIMETER)) * map_size.x
}
