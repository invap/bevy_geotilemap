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

/*!
GeoTilemap crate for the [Bevy game-engine](https://bevy.org/) to render slippy tile maps.

## Basic usage

Add the GeoTilemapPlugin to your bevy app, spawn a camera and a GeoTilemap component with your desired tile provider

```rust
use bevy::prelude::*;
use bevy_geotilemap::GeoTilemapPlugin;

let mut app = App::new();
app.add_plugins(GeoTilemapPlugin::default());
```
Then, just spawn an entity with the GeoTilemap component.
Notice that GeoTilemap needs to know which camera is rendering it, so that it can load the tiles in its viewport.
```rust
# use bevy::prelude::*;
# use bevy_geotilemap::GeoTilemap;
fn spawn_geotilemap(mut commands: Commands) {
    let camera = commands
        .spawn((
            Camera2d,
            Camera { ..default() }
        )).id();

    commands.spawn((
        GeoTilemap {
            camera: Some(camera),
            url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
            ..default()
        },
    ));
}
```
If you want a controllable map, similar to your typical web app, you can add the component ControllableMap:

```rust
# use bevy::prelude::*;
# use bevy_geotilemap::{GeoTilemap, controls::ControllableMap};
# fn spawn_geotilemap(mut commands: Commands) {
#     let camera = commands
#         .spawn((
#             Camera2d,
#             Camera { ..default() }
#         )).id();
    commands.spawn((
        GeoTilemap {
            camera: Some(camera),
            url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
            ..default()
        },
        ControllableMap::default()
    ));
# }
```

This will interact with your camera like you would expect: scroll to zoom, drag to move around.

The feature "markers", provides a simple way of adding markers at a specified longitude and latitude, by spawning a MapMarker component as child of your GeoTilemap entity:

```rust
# use bevy::prelude::*;
# use bevy_geotilemap::{GeoTilemap, marker::MapMarker};
# fn spawn_geotilemap(mut commands: Commands, asset_server: Res<AssetServer>) {
#     let camera = commands
#         .spawn((
#             Camera2d,
#             Camera { ..default() }
#         )).id();
    commands.spawn((
        GeoTilemap {
            camera: Some(camera),
            url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
            ..default()
        },
        children![
            Sprite {
                image: asset_server.load("pinpoint.png"),
                ..default()
            },
            MapMarker {
                latitude: 33.821_056,
                longitude: 71.945_98,
            },
        ]
    ));
# }
```

Checkout the examples to see different features available.

### Features
- Slippy tile dynamic rendering based on camera position.
- 2D camera controls.
- Tile caching using FIFO queue.
- Map markers for simple lat/lon placing of sprites and meshes.
- Customizable map size and position.
- Support for different tile providers as long as they follow [slippy tile zoom representation](https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames).

### Bugs/problems
- If you want to use an external tile server, you have to enable the feature "http"/"https" for bevy. Also if the tile provider uses jpeg, you need to add the feature "jpeg".
```bash
cargo add bevy --features http https jpeg
```
There is a known [issue](https://github.com/bevyengine/bevy/issues/10157) with bevy's web asset plugin, that tries to get metadata from the external source. To fix it you should disable meta checks from the asset plugin. See example "multiple_maps" for a quick fix.
*/
use bevy::{
    app::{App, HierarchyPropagatePlugin, Plugin, PreUpdate, Propagate, Update},
    camera::visibility::{RenderLayers, Visibility},
    ecs::{
        component::Component, entity::Entity, event::Event, lifecycle::HookContext,
        schedule::IntoScheduleConfigs, world::DeferredWorld,
    },
    math::Vec2,
    transform::components::Transform,
};

#[cfg(feature = "controls")]
use crate::controls::MapControlsPlugin;
#[cfg(feature = "markers")]
use crate::marker::MapMarkerPlugin;

use crate::{
    tile_cache::TileCachePlugin,
    tile_despawner::TileDespawner,
    tile_manager::{load_tile_map_in_view, redraw_event, spawn_tile_map_rect, update_zoom_level},
};

mod camera_properties;
#[cfg(feature = "controls")]
pub mod controls;
pub mod geo_coords;
mod load_checker;
#[cfg(feature = "markers")]
pub mod marker;
mod pipeline_ready;
mod tile_cache;
mod tile_despawner;
mod tile_manager;

/// Redraw everything event.
/// Very expensive! Only intended for occasional change in url provider.
#[derive(Event, Debug)]
pub struct RedrawMaps;

pub struct GeoTilemapPlugin {
    cache_size: usize,
    despawn_confirmation_frames: usize,
}

impl GeoTilemapPlugin {
    pub fn new(cache_size: usize, despawn_confirmation_frames: usize) -> Self {
        Self {
            cache_size,
            despawn_confirmation_frames,
        }
    }
}

impl Default for GeoTilemapPlugin {
    fn default() -> Self {
        Self {
            cache_size: 1024,
            despawn_confirmation_frames: 25,
        }
    }
}

impl Plugin for GeoTilemapPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "markers")]
        app.add_plugins(MapMarkerPlugin);

        #[cfg(feature = "controls")]
        app.add_plugins(MapControlsPlugin);

        if !app.is_plugin_added::<HierarchyPropagatePlugin<RenderLayers>>() {
            app.add_plugins(HierarchyPropagatePlugin::<RenderLayers>::new(PreUpdate));
        }

        app.add_plugins(TileCachePlugin::new(self.cache_size))
            .add_systems(Update, (update_zoom_level, load_tile_map_in_view).chain())
            .add_plugins(TileDespawner::new(self.despawn_confirmation_frames))
            .add_observer(spawn_tile_map_rect)
            .add_observer(redraw_event);
    }
}

fn set_render_layer(mut world: DeferredWorld, ctx: HookContext) {
    let geotilemap_entity = ctx.entity;

    let Some(geotilemap) = world.get::<GeoTilemap>(geotilemap_entity) else {
        return;
    };

    let Some(camera) = geotilemap.camera else {
        return;
    };

    let Some(render_layer) = world.get::<RenderLayers>(camera) else {
        return;
    };

    let render_layer = render_layer.clone();

    world
        .commands()
        .entity(geotilemap_entity)
        .insert(Propagate::<RenderLayers>(render_layer));
}

#[derive(Component, Debug, Clone)]
#[require(Transform, Visibility)]
#[component(on_add=set_render_layer)]
pub struct GeoTilemap {
    /// Rich url:
    ///     - '{x}' will be replaced by the value of the tile in x dimension
    ///     - '{y}' will be replaced by the value of the tile in y dimension
    ///     - '{z}' will be replaced by the value of the zoom level
    /// For example `https://tile.openstreetmap.org/{z}/{x}/{y}.png` for osm tile data
    pub url: String,
    pub camera: Option<Entity>,
    pub zoom_level: u32,
    /// Min zoom level for slippy tiles
    pub min_zoom_level: u32,
    /// Max zoom level for slippy tiles
    pub max_zoom_level: u32,
    pub zoom_policy: ZoomPolicy,
    pub custom_size: Option<Vec2>,
    /// Extends tile loading by a factor of the camera view
    pub loading_area_scale: f32,
}

impl GeoTilemap {
    pub fn default_size() -> Vec2 {
        Vec2::new(1024.0, 1024.0)
    }
}

impl Default for GeoTilemap {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
            camera: None,
            zoom_level: 0,
            min_zoom_level: 0,
            max_zoom_level: 19,
            zoom_policy: Default::default(),
            custom_size: Some(Self::default_size()),
            loading_area_scale: 1.5,
        }
    }
}

#[derive(Debug, Clone)]
/// Policy to trigger a tile map zoom event, based on number of visible tiles
pub struct ZoomPolicy {
    /// Number of tiles that must be visible to trigger a zoom-out in the smallest dimension
    pub max_tiles_in_axis: f32,

    /// Number of tiles that must be visible to trigger a zoom-in in the smallest dimension
    pub min_tiles_in_axis: f32,
}

impl Default for ZoomPolicy {
    fn default() -> Self {
        Self {
            max_tiles_in_axis: 4.0,
            min_tiles_in_axis: 1.0,
        }
    }
}
