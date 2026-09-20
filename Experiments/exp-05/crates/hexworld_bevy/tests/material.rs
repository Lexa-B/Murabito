//! The ground material's mode/tint resources reach the material asset — not the pixels,
//! just the plumbing (see `docs/plans/2026-09-20-exp-05-hex-voxel-chunks-plan.md`, Task 13).

mod common;

use bevy::prelude::*;
use common::headless_app;
use hexworld::StoreSettings;
use hexworld_bevy::{GroundMaterial, GroundMaterialHandle, LineMode, TintByLevel};

#[test]
fn line_mode_reaches_the_material() {
    let mut app = headless_app(StoreSettings::default());
    // Startup runs on the first update, which is when `GroundMaterialHandle` is created.
    app.update();
    app.world_mut().insert_resource(LineMode::Nested);
    app.update();

    let handle = app.world().resource::<GroundMaterialHandle>().0.clone();
    let materials = app.world().resource::<Assets<GroundMaterial>>();
    assert_eq!(materials.get(&handle).unwrap().extension.mode, 2);
}

#[test]
fn tint_by_level_reaches_the_material() {
    let mut app = headless_app(StoreSettings::default());
    app.update();
    app.world_mut().insert_resource(TintByLevel(true));
    app.update();

    let handle = app.world().resource::<GroundMaterialHandle>().0.clone();
    let materials = app.world().resource::<Assets<GroundMaterial>>();
    assert_eq!(materials.get(&handle).unwrap().extension.tint, 1);
}

#[test]
fn defaults_are_shaku_only_lines_and_no_tint() {
    let mut app = headless_app(StoreSettings::default());
    app.update();

    let handle = app.world().resource::<GroundMaterialHandle>().0.clone();
    let materials = app.world().resource::<Assets<GroundMaterial>>();
    let extension = &materials.get(&handle).unwrap().extension;
    assert_eq!(extension.mode, 1, "default line mode should be shaku-only");
    assert_eq!(extension.tint, 0, "tint should be off by default");
}
