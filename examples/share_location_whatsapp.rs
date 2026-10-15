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

//! In this example we show how to create create complex behaviour in the map.
//! Right click to trigger a drop down menu with two options:
//!     1. "Copy location", which will copy latitude and longitude in degrees to your clipboard (using arboard)
//!     2. "Share location in whatsapp", which will open a browser to share a google maps location of the current point over whatsapp.
//!
use std::time::Duration;

use arboard::Clipboard;
use bevy::{
    DefaultPlugins,
    app::{App, Startup},
    asset::{AssetServer, Assets, RenderAssetUsages},
    camera::{
        Camera, Camera2d, OrthographicProjection, Projection, RenderTarget, ScalingMode,
        visibility::{RenderLayers, Visibility},
    },
    color::Color,
    ecs::{
        children,
        component::Component,
        entity::Entity,
        hierarchy::ChildSpawnerCommands,
        observer::On,
        query::With,
        resource::Resource,
        system::{Commands, Query, Res, ResMut, Single},
    },
    image::Image,
    log::{error, warn},
    math::Vec2,
    picking::{
        events::{PointerClick, PointerDragLeave, PointerPress, PointerRelease, PointerScroll},
        pointer::PointerButton,
    },
    render::render_resource::{TextureDimension, TextureFormat, TextureUsages},
    sprite::{Sprite, Text2d},
    text::{FontSize, TextColor, TextFont},
    transform::components::{GlobalTransform, Transform},
    ui::{
        BackgroundColor, BorderColor, BorderRadius, BoxShadow, ComputedNode, Display,
        FlexDirection, Node, PositionType, ShadowStyle, UiRect, percent, px,
        widget::{Text, ViewportNode},
    },
    utils::default,
};
use bevy_geotilemap::{
    GeoTilemap, GeoTilemapPlugin,
    controls::ControllableMap,
    geo_coords::{lat_lon_to_world, mts_to_tiles, world_to_lat_lon},
    marker::{MapMarker, RelativeMarker},
};
use urlencoding::encode;

#[derive(Component)]
pub struct ResizableMarker {
    pub scale: Vec2,
}

#[derive(Resource)]
pub struct SelectedLocation(pub Option<Vec2>);

#[derive(Resource)]
pub struct ClipboardResource(pub Clipboard);

#[derive(Component)]
pub struct AppContainer;

const LAT_DEG: f32 = -41.124_397;
const LON_DEG: f32 = -71.242_38;

fn main() {
    let mut app = App::new();

    let clipboard = Clipboard::new().unwrap();

    app.insert_resource(ClipboardResource(clipboard))
        .insert_resource(SelectedLocation(None))
        .add_plugins(DefaultPlugins)
        .add_plugins(GeoTilemapPlugin::default())
        .add_systems(Startup, spawn_map_app);

    app.run();
}

fn spawn_map_app(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let initial_location = lat_lon_to_world(LAT_DEG, LON_DEG, GeoTilemap::default_size());
    let initial_scale = mts_to_tiles(3_000.0, LAT_DEG, GeoTilemap::default_size());

    commands.spawn((Camera2d, Camera::default()));

    commands.spawn((
        Text::new("Right click to open menu"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

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

    commands
        .spawn((
            AppContainer,
            Node {
                top: percent(10),
                left: percent(10),
                width: percent(80),
                height: percent(80),
                ..default()
            },
            Transform::default(),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .with_children(|commands| {
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    border_radius: BorderRadius::all(px(10)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                ViewportNode::new(map_camera),
            ));

            spawn_right_click_menu(commands);
        });

    commands
        .spawn((
            GeoTilemap {
                camera: Some(map_camera),
                url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
                ..default()
            },
            Transform::default(),
            Visibility::default(),
            ControllableMap::default(),
            RenderLayers::layer(1),
        ))
        .with_child((
            children![(
                Visibility::Visible,
                Sprite {
                    image: asset_server.load("pinpoint.png"),
                    custom_size: Some(Vec2::new(1.0, 1.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.5, 0.0)
            )],
            Visibility::Hidden,
            MapMarker {
                latitude: 0.0,
                longitude: 0.0,
            },
            Transform::from_xyz(0.0, 0.0, 2.0),
            RelativeMarker {
                scale: Vec2::new(0.05, 0.05),
                ..default()
            },
        ))
        .observe(place_marker)
        .observe(hide_rightclick_menu_press)
        .observe(hide_rightclick_menu_scroll);
}

pub fn place_marker(
    pointer: On<PointerClick>,
    mut commands: Commands,
    geotilemap: Single<(Entity, &GeoTilemap)>,
    mut marker: Single<(Entity, &mut MapMarker)>,
    cameras: Query<(&GlobalTransform, &Camera)>,
    mut rightclick_menu: Single<(Entity, &mut Node), With<RightClickMenu>>,
    app_container: Single<&ComputedNode, With<AppContainer>>,
) {
    if let Some(camera_entity) = geotilemap.1.camera
        && let Ok((global_transform, camera)) = cameras.get(camera_entity)
        && let Ok(location) =
            camera.viewport_to_world_2d(global_transform, pointer.pointer.location().position)
        && pointer.button == PointerButton::Secondary
    {
        let degrees = world_to_lat_lon(location, GeoTilemap::default_size());

        commands.entity(marker.0).insert(Visibility::Visible);
        marker.1.latitude = degrees.x;
        marker.1.longitude = degrees.y;

        commands.insert_resource(SelectedLocation(Some(degrees)));

        commands
            .entity(rightclick_menu.0)
            .insert(Visibility::Visible);

        let container_width = (app_container.size[0]
            - (app_container.padding.max_inset.x + app_container.padding.max_inset.x))
            * app_container.inverse_scale_factor;
        let container_height = (app_container.size[1]
            - (app_container.padding.max_inset.y + app_container.padding.max_inset.y))
            * app_container.inverse_scale_factor;

        if pointer.pointer.location().position.x + 150.0 > container_width {
            rightclick_menu.1.left = px(pointer.pointer.location().position.x - 150.0);
        } else {
            rightclick_menu.1.left = px(pointer.pointer.location().position.x);
        }

        if pointer.pointer.location().position.y + 50.0 > container_height {
            rightclick_menu.1.top = px(pointer.pointer.location().position.y - 50.0);
        } else {
            rightclick_menu.1.top = px(pointer.pointer.location().position.y);
        }
    } else if pointer.duration < Duration::from_millis(100) {
        commands.entity(marker.0).insert(Visibility::Hidden);
        commands.insert_resource(SelectedLocation(None));
    }
}

pub fn hide_rightclick_menu_press(
    pointer: On<PointerPress>,
    mut commands: Commands,
    rightclick_menu: Single<(Entity, &Node), With<RightClickMenu>>,
) {
    if pointer.button == PointerButton::Primary {
        commands
            .entity(rightclick_menu.0)
            .insert(Visibility::Hidden);
    }
}

pub fn hide_rightclick_menu_scroll(
    _: On<PointerScroll>,
    mut commands: Commands,
    rightclick_menu: Single<(Entity, &Node), With<RightClickMenu>>,
) {
    commands
        .entity(rightclick_menu.0)
        .insert(Visibility::Hidden);
}

#[derive(Component)]
pub struct RightClickMenu;

fn spawn_right_click_menu(commands: &mut ChildSpawnerCommands) {
    commands
        .spawn((
            RightClickMenu,
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: px(150),
                height: px(50),
                border_radius: BorderRadius::all(px(2)),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            Visibility::Hidden,
            Transform::default(),
            BoxShadow(vec![ShadowStyle {
                color: Color::srgb(0.5, 0.5, 0.5),
                x_offset: px(2),
                y_offset: px(2),
                spread_radius: px(1),
                blur_radius: px(1),
            }]),
            BackgroundColor(Color::srgb(0.8, 0.8, 0.8)),
        ))
        .with_children(|commands| {
            spawn_copy_location_button(commands);

            spawn_share_button(commands);
        });
}

fn spawn_copy_location_button(commands: &mut ChildSpawnerCommands) {
    commands
        .spawn((
            Transform::default(),
            Visibility::default(),
            Node {
                height: percent(50),
                border: UiRect {
                    left: px(0),
                    right: px(0),
                    top: px(0),
                    bottom: px(1),
                },
                padding: UiRect::all(px(5)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
            children![(
                Transform::default(),
                Visibility::default(),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                Text2d("Copy location".to_string()),
                TextColor(Color::srgb(0.1, 0.1, 0.1)),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
            )],
        ))
        .observe(click_copy_button)
        .observe(press_button)
        .observe(release_button)
        .observe(drag_leave_button);
}

fn spawn_share_button(commands: &mut ChildSpawnerCommands) {
    commands
        .spawn((
            Transform::default(),
            Visibility::default(),
            Node {
                height: percent(50),
                border: UiRect {
                    left: px(0),
                    right: px(0),
                    top: px(1),
                    bottom: px(0),
                },
                padding: UiRect::all(px(5)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.3, 0.3, 0.3)),
            children![(
                Transform::default(),
                Visibility::default(),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                Text2d("Share in Whatsapp".to_string()),
                TextColor(Color::srgb(0.1, 0.1, 0.1)),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
            )],
        ))
        .observe(click_share_button)
        .observe(press_button)
        .observe(release_button)
        .observe(drag_leave_button);
}

fn press_button(press: On<PointerPress>, node: Query<Entity, With<Node>>, mut commands: Commands) {
    if press.button == PointerButton::Primary
        && let Ok(node_entity) = node.get(press.entity)
    {
        commands
            .entity(node_entity)
            .insert(BackgroundColor(Color::srgb(0.5, 0.5, 0.5)));
    }
}

fn release_button(
    release: On<PointerRelease>,
    node: Query<Entity, With<Node>>,
    mut commands: Commands,
) {
    if release.button == PointerButton::Primary
        && let Ok(node_entity) = node.get(release.entity)
    {
        commands
            .entity(node_entity)
            .insert(BackgroundColor(Color::srgb(0.8, 0.8, 0.8)));
    }
}

fn drag_leave_button(
    drag_leave: On<PointerDragLeave>,
    node: Query<Entity, With<Node>>,
    mut commands: Commands,
) {
    if drag_leave.button == PointerButton::Primary
        && let Ok(node_entity) = node.get(drag_leave.entity)
    {
        commands
            .entity(node_entity)
            .insert(BackgroundColor(Color::srgb(0.8, 0.8, 0.8)));
    }
}

fn click_share_button(click: On<PointerClick>, selected_location: Res<SelectedLocation>) {
    if click.button == PointerButton::Primary
        && let SelectedLocation(Some(degrees)) = *selected_location
    {
        let google_maps_link = format!("https://maps.google.com/?q={},{}", degrees.x, degrees.y);
        let url = format!("https://wa.me/?text={}", encode(&google_maps_link));
        if webbrowser::open(&url).is_err() {
            warn!("Could not open web browser");
        }
    }
}

fn click_copy_button(
    click: On<PointerClick>,
    selected_location: Res<SelectedLocation>,
    mut clipboard: ResMut<ClipboardResource>,
) {
    if click.button == PointerButton::Primary
        && let SelectedLocation(Some(degrees)) = *selected_location
    {
        let text = format!("{} {}", degrees.x, degrees.y);
        if let Err(e) = clipboard.0.set_text(text) {
            error!("{e}");
        }
    }
}
