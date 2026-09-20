//! The egui side panel: the address under the focus and the mouse, per-level load stats
//! and triangle counts, live tuning, display toggles, FPS, and the debug loader list.
//!
//! egui 0.36 has no `SidePanel`; panels are `egui::Panel::right(id)` and take a root `Ui`,
//! not a context — see the plan's "Bevy notes" section. This module also owns the hover
//! system that finds the cell under the mouse, since the panel is the only thing that
//! reads it.

use std::collections::HashMap;

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use hexworld::{address_of, cell_centre_m, round_at, Hex, Level, WorldConfig};

use hexworld_bevy::axes::{from_bevy, to_bevy};
use hexworld_bevy::{ChunkView, HexWorld, LineMode, Loader, TintByLevel};

use crate::camera::CameraRig;

/// The cell under the mouse and the layer it is in, found by [`update_hover`]. `None`
/// when the cursor is off the window, or the pick ray does not hit the ground plane at
/// all (no window, no camera, or the focus has no transform yet).
#[derive(Resource, Default)]
pub struct HoveredCell(pub Option<(Hex, i32)>);

/// Extra `Loader` entities the panel's own "add" button has spawned, oldest first, so
/// "remove the last" has an unambiguous target. The focus's own loader (the one that
/// also carries `CameraRig`) is never in this list and is never touched by the button.
#[derive(Resource, Default)]
pub struct ExtraLoaders(Vec<Entity>);

/// Each shown chunk entity's triangle count, captured by [`track_triangle_counts`] the
/// same frame its `Mesh3d` is set, before that frame's render extraction discards the
/// CPU-side copy.
///
/// Chunk meshes are built with `RenderAssetUsages::RENDER_WORLD` only (`entities::
/// to_bevy_mesh`), so their vertex data is not kept on the CPU past the frame it is
/// uploaded — reading `Assets<Mesh>` for an already-shown chunk from `panel::draw` (which
/// runs in `EguiPrimaryContextPass`, after extraction) panics: "Mesh has been extracted to
/// RenderWorld", found by running the viewer live. Re-deriving a count without touching a
/// `Mesh` at all would mean duplicating `hexworld::mesh`'s geometry logic, and widening
/// `to_bevy_mesh`'s asset usage to keep every chunk's CPU-side mesh data around permanently
/// is a `hexworld_bevy` change outside what the panel's brief authorises. Instead, this
/// resource is written from `Update` (ordered after `HexWorldSet`, so a chunk's `Mesh3d` is
/// already current for the frame), while the data is still there to read.
#[derive(Resource, Default)]
pub struct TriangleCounts(HashMap<Entity, usize>);

impl TriangleCounts {
    fn get(&self, entity: Entity) -> Option<usize> {
        self.0.get(&entity).copied()
    }
}

/// Capture the triangle count of every chunk whose `Mesh3d` changed this frame (inserted
/// or swapped by a handover), and drop counts for chunks that are no longer shown. Must
/// run after `HexWorldSet` and before the render app's extraction — see `TriangleCounts`.
pub fn track_triangle_counts(
    changed: Query<(Entity, &Mesh3d), Changed<Mesh3d>>,
    shown: Query<Entity, With<ChunkView>>,
    meshes: Res<Assets<Mesh>>,
    mut counts: ResMut<TriangleCounts>,
) {
    for (entity, mesh3d) in &changed {
        let Some(mesh) = meshes.get(&mesh3d.0) else {
            continue;
        };
        // `try_indices_option`, not `indices`: the latter panics under the same
        // RenderWorld-extraction condition this whole resource exists to avoid, and a
        // false assumption about scheduling order should degrade to "no count yet"
        // rather than crash the viewer.
        let Ok(indices) = mesh.try_indices_option() else {
            continue;
        };
        let triangles = indices.map(|i| i.len() / 3).unwrap_or(0);
        counts.0.insert(entity, triangles);
    }
    let alive: std::collections::HashSet<Entity> = shown.iter().collect();
    counts.0.retain(|entity, _| alive.contains(entity));
}

/// The cell under the mouse, found by casting a ray onto the plane at the focus height.
pub fn update_hover(
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    focus: Query<&GlobalTransform, With<CameraRig>>,
    mut hovered: ResMut<HoveredCell>,
    world: Res<HexWorld>,
) {
    hovered.0 = None;
    let (Ok(window), Ok((camera, camera_transform)), Ok(focus)) =
        (windows.single(), cameras.single(), focus.single())
    else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        return;
    };
    let ground = focus.translation();
    let Some(distance) = ray.intersect_plane(ground, InfinitePlane3d::new(Vec3::Y)) else {
        return;
    };
    let point = ray.get_point(distance);
    let (east, north, _) = from_bevy(point);
    let shaku = round_at(east, north, Level::Shaku);
    let layer = world
        .store()
        .surface_height_m(east, north)
        .map(|h| world.store().config().layer_of_height(h))
        .unwrap_or(0);
    hovered.0 = Some((shaku, layer));
}

/// The layer under a point, from the finest terrain loaded there — 0 if nothing is
/// loaded yet, matching `update_hover`'s own fallback.
fn layer_at(world: &HexWorld, east: f64, north: f64) -> i32 {
    world
        .store()
        .surface_height_m(east, north)
        .map(|h| world.store().config().layer_of_height(h))
        .unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
pub fn draw(
    mut contexts: EguiContexts,
    mut world: ResMut<HexWorld>,
    mut line_mode: ResMut<LineMode>,
    mut tint: ResMut<TintByLevel>,
    mut focus: Query<(Entity, &CameraRig, &mut Loader)>,
    other_loaders: Query<(Entity, &GlobalTransform, &Loader), Without<CameraRig>>,
    chunks: Query<(Entity, &ChunkView)>,
    triangle_counts: Res<TriangleCounts>,
    hovered: Res<HoveredCell>,
    diagnostics: Res<DiagnosticsStore>,
    mut extra_loaders: ResMut<ExtraLoaders>,
    mut commands: Commands,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    egui::Panel::right("hexworld_panel")
        .default_size(340.0)
        .show(&mut viewport_ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                draw_address(ui, &world, &focus, &hovered);
                ui.separator();
                draw_levels(ui, &world, &chunks, &triangle_counts);
                ui.separator();
                draw_tuning(ui, &mut world, &mut focus);
                ui.separator();
                draw_display(ui, &mut line_mode, &mut tint);
                ui.separator();
                draw_fps(ui, &diagnostics);
                ui.separator();
                draw_loaders(
                    ui,
                    &world,
                    &focus,
                    &other_loaders,
                    &hovered,
                    &mut extra_loaders,
                    &mut commands,
                );
            });
        });
    Ok(())
}

fn draw_address(
    ui: &mut egui::Ui,
    world: &HexWorld,
    focus: &Query<(Entity, &CameraRig, &mut Loader)>,
    hovered: &HoveredCell,
) {
    ui.heading("Address");
    if let Some((_, rig, _)) = focus.iter().next() {
        let shaku = round_at(rig.focus_east, rig.focus_north, Level::Shaku);
        let layer = layer_at(world, rig.focus_east, rig.focus_north);
        let addr = address_of(shaku, layer);
        ui.label(format!("Focus: {addr}"));
        ui.label(format!(
            "  at ({:.2}, {:.2}) m",
            rig.focus_east, rig.focus_north
        ));
    } else {
        ui.label("Focus: (no camera rig)");
    }
    match hovered.0 {
        Some((shaku, layer)) => {
            let addr = address_of(shaku, layer);
            let (east, north) = cell_centre_m(shaku, Level::Shaku);
            ui.label(format!("Hover: {addr}"));
            ui.label(format!("  at ({east:.2}, {north:.2}) m"));
        }
        None => {
            ui.label("Hover: (nothing under the mouse)");
        }
    }
}

fn draw_levels(
    ui: &mut egui::Ui,
    world: &HexWorld,
    chunks: &Query<(Entity, &ChunkView)>,
    triangle_counts: &TriangleCounts,
) {
    ui.heading("Levels");
    // Triangle counts are not something the store tracks (it only knows chunks and
    // columns); they come from `TriangleCounts`, grouped by the level each shown chunk's
    // `ChunkKey` names — see that resource's doc comment for why the count is cached
    // rather than read straight off the entity's `Mesh3d` here.
    let mut triangles = [0usize; 5];
    for (entity, view) in chunks {
        if let Some(count) = triangle_counts.get(entity) {
            triangles[view.0.level as usize] += count;
        }
    }
    let stats = world.store().stats();
    // `stats.per_level` is keyed by *chunk* level (a `ChunkKey`'s level is its parent's
    // level; the chunk holds columns for cells one level finer, at `level.child()` — see
    // `ChunkKey`'s doc comment). No chunk is ever keyed at `Level::Shaku`, so we walk the
    // chunk levels that actually occur and label each row by the detail it provides.
    const CHUNK_LEVELS: [Level; 4] = [Level::Ken, Level::Cho, Level::Ri, Level::World];
    egui::Grid::new("level_stats").striped(true).show(ui, |ui| {
        ui.label("detail");
        ui.label("chunks");
        ui.label("columns");
        ui.label("lingering");
        ui.label("loads/s");
        ui.label("unloads/s");
        ui.label("triangles");
        ui.end_row();
        for chunk_level in CHUNK_LEVELS {
            let detail_level = chunk_level
                .child()
                .expect("chunk levels always have a child");
            let s = stats.per_level[chunk_level as usize];
            ui.label(detail_level.name());
            ui.label(s.chunks.to_string());
            ui.label(s.columns.to_string());
            ui.label(s.lingering.to_string());
            ui.label(s.loads_last_second.to_string());
            ui.label(s.unloads_last_second.to_string());
            ui.label(triangles[chunk_level as usize].to_string());
            ui.end_row();
        }
    });
}

fn draw_tuning(
    ui: &mut egui::Ui,
    world: &mut HexWorld,
    focus: &mut Query<(Entity, &CameraRig, &mut Loader)>,
) {
    ui.heading("Tuning");
    if let Some((_, _, mut loader)) = focus.iter_mut().next() {
        ui.label("Rings (focus loader)");
        ui.add(egui::Slider::new(&mut loader.rings.shaku, 0..=24).text("shaku"));
        ui.add(egui::Slider::new(&mut loader.rings.ken, 0..=24).text("ken"));
        ui.add(egui::Slider::new(&mut loader.rings.cho, 0..=24).text("cho"));
    } else {
        ui.label("Rings: (no camera rig)");
    }

    let mut delay = world.store().settings().unload_delay_s;
    if ui
        .add(egui::Slider::new(&mut delay, 0.0..=30.0).text("unload delay (s)"))
        .changed()
    {
        world.store_mut().settings_mut().unload_delay_s = delay;
    }

    // The seed and the layer thickness are baked into every generated chunk, so changing
    // either replaces the world wholesale (`HexWorld::set_config`) rather than editing it
    // in place — there is no way to reconcile chunks generated under the old values with
    // the new ones.
    let cfg = *world.store().config();
    let mut thickness = cfg.layer_thickness_sun;
    let thickness_response =
        ui.add(egui::Slider::new(&mut thickness, 1.0..=30.0).text("layer thickness (sun)"));
    let mut seed = cfg.seed;
    let seed_response = ui.add(
        egui::DragValue::new(&mut seed)
            .range(0..=u64::MAX)
            .prefix("seed "),
    );
    // A world rebuild is not cheap — it drops and regenerates everything — so these two
    // commit once per gesture (the drag being released, or a typed value losing focus)
    // rather than on every frame the widget merely reports `changed()`, which fires on
    // every tick of a drag. The rings and unload-delay controls above stay live on every
    // change: they only mutate a `Loader`/`StoreSettings` field in place, cheap either way.
    let thickness_committed = thickness_response.drag_stopped() || thickness_response.lost_focus();
    let seed_committed = seed_response.drag_stopped() || seed_response.lost_focus();
    if thickness_committed || seed_committed {
        world.set_config(WorldConfig {
            layer_thickness_sun: thickness,
            seed,
            ..cfg
        });
    }
}

fn draw_display(ui: &mut egui::Ui, line_mode: &mut LineMode, tint: &mut TintByLevel) {
    ui.heading("Display");
    ui.horizontal(|ui| {
        ui.radio_value(line_mode, LineMode::Off, "off");
        ui.radio_value(line_mode, LineMode::ShakuOnly, "shaku only");
        ui.radio_value(line_mode, LineMode::Nested, "nested");
    });
    ui.checkbox(&mut tint.0, "tint by level");
}

fn draw_fps(ui: &mut egui::Ui, diagnostics: &DiagnosticsStore) {
    ui.heading("Performance");
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed());
    let frame_time = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed());
    match (fps, frame_time) {
        (Some(fps), Some(ms)) => {
            ui.label(format!("{fps:.0} fps ({ms:.2} ms)"));
        }
        _ => {
            ui.label("fps: (warming up)");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_loaders(
    ui: &mut egui::Ui,
    world: &HexWorld,
    focus: &Query<(Entity, &CameraRig, &mut Loader)>,
    other_loaders: &Query<(Entity, &GlobalTransform, &Loader), Without<CameraRig>>,
    hovered: &HoveredCell,
    extra_loaders: &mut ExtraLoaders,
    commands: &mut Commands,
) {
    ui.heading("Loaders");
    egui::Grid::new("loader_rows").striped(true).show(ui, |ui| {
        ui.label("entity");
        ui.label("address");
        ui.end_row();
        if let Some((entity, rig, _)) = focus.iter().next() {
            let shaku = round_at(rig.focus_east, rig.focus_north, Level::Shaku);
            let layer = layer_at(world, rig.focus_east, rig.focus_north);
            ui.label(format!("{entity:?} (focus)"));
            ui.label(format!("{}", address_of(shaku, layer)));
            ui.end_row();
        }
        for (entity, transform, _) in other_loaders {
            let (east, north, _) = from_bevy(transform.translation());
            let shaku = round_at(east, north, Level::Shaku);
            let layer = layer_at(world, east, north);
            ui.label(format!("{entity:?}"));
            ui.label(format!("{}", address_of(shaku, layer)));
            ui.end_row();
        }
    });

    ui.horizontal(|ui| {
        let can_add = hovered.0.is_some();
        if ui
            .add_enabled(can_add, egui::Button::new("Add loader at mouse"))
            .clicked()
        {
            if let Some((shaku, _)) = hovered.0 {
                let (east, north) = cell_centre_m(shaku, Level::Shaku);
                let entity = commands
                    .spawn((
                        Transform::from_translation(to_bevy(east, north, 0.0)),
                        Loader::default(),
                    ))
                    .id();
                extra_loaders.0.push(entity);
            }
        }
        let can_remove = !extra_loaders.0.is_empty();
        if ui
            .add_enabled(can_remove, egui::Button::new("Remove last loader"))
            .clicked()
        {
            if let Some(entity) = extra_loaders.0.pop() {
                commands.entity(entity).despawn();
            }
        }
    });
}
