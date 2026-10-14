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
    prelude::*,
    render::{render_resource::*, *},
};

pub struct PipelinesReadyPlugin;
impl Plugin for PipelinesReadyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PipelinesReady::default());

        // In order to gain access to the pipelines status, we have to
        // go into the `RenderApp`, grab the resource from the main App
        // and then update the pipelines status from there.
        // Writing between these Apps can only be done through the
        // `ExtractSchedule`.
        app.sub_app_mut(RenderApp)
            .add_systems(ExtractSchedule, update_pipelines_ready);
    }
}

#[derive(Resource, Debug, Default)]
pub struct PipelinesReady(pub bool);

fn update_pipelines_ready(mut main_world: ResMut<MainWorld>, pipelines: Res<PipelineCache>) {
    if let Some(mut pipelines_ready) = main_world.get_resource_mut::<PipelinesReady>() {
        pipelines_ready.0 = pipelines.waiting_pipelines().count() == 0;
    }
}
