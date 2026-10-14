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

//! In this example we show how removing the component [`ControllableMap`] blocks the camera in position
use bevy::{
    DefaultPlugins,
    app::{App, Startup},
    asset::{Assets, RenderAssetUsages},
    camera::{
        Camera, Camera2d, OrthographicProjection, Projection, RenderTarget, ScalingMode,
        visibility::RenderLayers,
    },
    color::Color,
    ecs::{
        component::Component,
        entity::Entity,
        observer::On,
        query::With,
        system::{Commands, Query, ResMut},
    },
    image::Image,
    picking::events::PointerClick,
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
    sprite::Text2d,
    transform::components::Transform,
    ui::{BackgroundColor, BorderRadius, Node, PositionType, percent, px, widget::ViewportNode},
    ui_widgets::Button,
    utils::default,
};
use bevy_geotilemap::{
    GeoTilemap, GeoTilemapPlugin,
    controls::ControllableMap,
    geo_coords::{lat_lon_to_world, mts_to_tiles},
};

#[derive(Component)]
pub struct BlockMap(pub Entity);

const LAT_DEG: f32 = -41.124_397;
const LON_DEG: f32 = -71.242_38;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(GeoTilemapPlugin::default())
        .add_systems(Startup, spawn_map_app);

    app.run();
}

fn spawn_map_app(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands.spawn((Camera2d, Camera::default()));

    // Set up an texture for the 2D camera to render to.
    // The size of the texture will be based on the viewport's ui size.
    let mut image = Image::new_uninit(
        default(),
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::all(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;
    let image_handle = images.add(image);

    let initial_location = lat_lon_to_world(LAT_DEG, LON_DEG, GeoTilemap::default_size());
    let initial_scale = mts_to_tiles(3_000.0, LAT_DEG, GeoTilemap::default_size());

    // Spawn the 2D camera
    let map_camera = commands
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
            RenderTarget::Image(image_handle.clone().into()),
            RenderLayers::layer(1),
        ))
        .id();

    let geotilemap = commands
        .spawn((
            GeoTilemap {
                camera: Some(map_camera),
                url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
                ..default()
            },
            ControllableMap::default(),
            RenderLayers::layer(1),
        ))
        .id();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(80),
                height: percent(80),
                top: percent(10),
                left: percent(10),
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            Transform::default(),
            ViewportNode::new(map_camera),
        ))
        .with_children(|commands| {
            commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: px(140),
                        height: px(30),
                        top: px(10),
                        border_radius: BorderRadius::all(px(3)),
                        right: px(10),
                        ..default()
                    },
                    Button,
                    BlockMap(geotilemap),
                    BackgroundColor(Color::srgb(0.7, 0.1, 0.1)),
                    Text2d("BLOCK MAP".to_string()),
                ))
                .observe(block_map);
        });
}

fn block_map(
    button_click: On<PointerClick>,
    mut commands: Commands,
    geotilemaps: Query<Option<&ControllableMap>, With<GeoTilemap>>,
    block_maps: Query<(Entity, &BlockMap)>,
) {
    if let Ok((button_entity, block_map)) = block_maps.get(button_click.entity) {
        if let Ok(Some(_)) = geotilemaps.get(block_map.0) {
            commands.entity(block_map.0).remove::<ControllableMap>();
            commands.entity(button_entity).insert((
                BackgroundColor(Color::srgb(0.1, 0.7, 0.1)),
                Text2d("UNBLOCK MAP".to_string()),
            ));
        } else {
            commands
                .entity(block_map.0)
                .insert(ControllableMap::default());
            commands.entity(button_entity).insert((
                BackgroundColor(Color::srgb(0.7, 0.1, 0.1)),
                Text2d("BLOCK MAP".to_string()),
            ));
        }
    }
}
