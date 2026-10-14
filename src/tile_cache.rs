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

//! Tile caching for slippy tiles, saves image handles for quick
//! asset loading.
use bevy::{
    app::{App, Plugin, Update},
    asset::{AssetLoadFailedEvent, AssetServer, Handle},
    ecs::{message::MessageReader, resource::Resource, system::ResMut},
    image::Image,
    log::{debug, error, warn},
    platform::collections::HashMap,
};
use std::collections::VecDeque;

pub struct TileCachePlugin {
    cache_size: usize,
}

impl TileCachePlugin {
    pub fn new(cache_size: usize) -> Self {
        Self { cache_size }
    }
}

impl Default for TileCachePlugin {
    fn default() -> Self {
        Self { cache_size: 1024 }
    }
}

impl Plugin for TileCachePlugin {
    fn build(&self, app: &mut App) {
        if let Some(asset_server) = app.world().get_resource::<AssetServer>() {
            app.insert_resource(TileCache::new(self.cache_size, asset_server.clone()))
                .add_systems(Update, cleanup_tile_cache);
        } else {
            error!("Asset server resource not found");
        }
    }
}

/// Tile caching resource, with a FIFO replacing strategy
#[derive(Resource)]
pub struct TileCache {
    /// Cache size
    cache_size: usize,
    // pub path: String,
    /// Asset server reference
    asset_server: AssetServer,
    /// Hashmap to store handle images using
    /// (x y z) slippy tile coordinates as keys
    hash: HashMap<(u32, u32, u32, String), Handle<Image>>,
    /// FIFO to determine which tile to remove from the cache.
    fifo_replace_policy: VecDeque<(u32, u32, u32, String)>,
}

impl TileCache {
    pub fn new(cache_size: usize, asset_server: AssetServer) -> Self {
        Self {
            cache_size,
            asset_server,
            hash: HashMap::new(),
            fifo_replace_policy: VecDeque::new(),
        }
    }

    /// Load image asset using (x y z) coordinate as key
    pub fn load(&mut self, x: u32, y: u32, z: u32, url: &str) -> Handle<Image> {
        if x >= 2_u32.pow(z) || y >= 2_u32.pow(z) {
            error!("Tile map out of bounds: x: {} y: {} z: {}", x, y, z);
        }

        let formatted_url = url
            .replace("{x}", &format!("{x}"))
            .replace("{-x}", &format!("{}", 2_u32.pow(z) - 1 - x))
            .replace("{y}", &format!("{y}"))
            .replace("{-y}", &format!("{}", 2_u32.pow(z) - 1 - y))
            .replace("{z}", &format!("{z}"));

        // Check if image is saved
        if let Some(handle) = self.hash.get(&(x, y, z, formatted_url.clone())) {
            handle.clone()
        } else {
            // Load image from server
            let handle = self.asset_server.load(formatted_url.clone());

            // Update cache using FIFO strategy
            if self.fifo_replace_policy.len() + 1 == self.cache_size
                && let Some(coord) = self.fifo_replace_policy.pop_front()
            {
                self.hash.remove(&coord);
            }

            let new_coord = (x, y, z, formatted_url);
            self.hash.insert(new_coord.clone(), handle.clone());
            self.fifo_replace_policy.push_back(new_coord);

            debug!("Cache size: {}", self.fifo_replace_policy.len());

            handle
        }
    }

    pub fn clear(&mut self) {
        self.hash.clear();
        self.fifo_replace_policy.clear();
    }
}

fn cleanup_tile_cache(
    mut failed_events: MessageReader<AssetLoadFailedEvent<Image>>,
    mut tile_cache: ResMut<TileCache>,
) {
    for failed_tile in failed_events.read() {
        warn!("Failed request: removing from cache.");
        tile_cache.hash = tile_cache
            .hash
            .clone()
            .into_iter()
            .filter(|(_, v)| v.id() != failed_tile.id)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::io::AssetReaderError;
    use bevy::asset::{AssetLoadFailedEvent, AssetPath};
    use bevy::prelude::*;

    fn setup_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_plugins(ImagePlugin::default())
            .add_message::<AssetLoadFailedEvent<Image>>()
            .add_systems(Update, cleanup_tile_cache);

        // TileCache needs AssetServer, which is only available after plugins are built
        let asset_server = app.world().resource::<AssetServer>().clone();
        app.insert_resource(TileCache {
            cache_size: 10,
            asset_server,
            hash: HashMap::new(),
            fifo_replace_policy: VecDeque::new(),
        });

        app
    }

    fn insert_tile(app: &mut App, key: (u32, u32, u32, String)) -> Handle<Image> {
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());

        app.world_mut()
            .resource_mut::<TileCache>()
            .hash
            .insert(key, handle.clone());

        handle
    }

    fn send_failed_event(app: &mut App, handle: &Handle<Image>) {
        app.world_mut()
            .write_message(AssetLoadFailedEvent::<Image> {
                id: handle.id(),
                path: AssetPath::from("mock/path.png"),
                error: AssetReaderError::HttpError(404).into(),
            });
    }

    #[test]
    fn expected_removes_only_failed_tile() {
        let mut app = setup_app();
        let _good = insert_tile(&mut app, (0, 0, 0, "my-url".to_string()));
        let bad = insert_tile(&mut app, (1, 1, 1, "my-url".to_string()));

        send_failed_event(&mut app, &bad);
        app.update();

        let cache = app.world().resource::<TileCache>();
        assert_eq!(cache.hash.len(), 1);
        assert!(
            cache.hash.contains_key(&(0, 0, 0, "my-url".to_string())),
            "good tile should remain"
        );
        assert!(
            !cache.hash.contains_key(&(1, 1, 1, "my-url".to_string())),
            "bad tile should be removed"
        );
    }

    #[test]
    fn expected_multiple_failures_remove_each_tile() {
        let mut app = setup_app();
        let tile_a = insert_tile(&mut app, (0, 0, 0, "my-url".to_string()));
        let tile_b = insert_tile(&mut app, (1, 1, 1, "my-url".to_string()));
        let _tile_c = insert_tile(&mut app, (2, 2, 2, "my-url".to_string()));

        send_failed_event(&mut app, &tile_a);
        send_failed_event(&mut app, &tile_b);
        app.update();

        let cache = app.world().resource::<TileCache>();
        assert!(!cache.hash.contains_key(&(0, 0, 0, "my-url".to_string())));
        assert!(!cache.hash.contains_key(&(1, 1, 1, "my-url".to_string())));
        assert!(
            cache.hash.contains_key(&(2, 2, 2, "my-url".to_string())),
            "tile_c did not fail, should remain"
        );
        assert_eq!(cache.hash.len(), 1);
    }

    #[test]
    fn no_events_does_not_modify_cache() {
        let mut app = setup_app();
        insert_tile(&mut app, (0, 0, 0, "my-url".to_string()));
        insert_tile(&mut app, (1, 1, 1, "my-url".to_string()));

        app.update(); // no messages fired

        let cache = app.world().resource::<TileCache>();
        assert_eq!(
            cache.hash.len(),
            2,
            "cache should not change without messages"
        );
    }

    #[test]
    fn empty_cache_does_not_panic() {
        let mut app = setup_app();
        let dummy = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());

        send_failed_event(&mut app, &dummy);
        app.update();

        let cache = app.world().resource::<TileCache>();
        assert!(cache.hash.is_empty());
    }
}
