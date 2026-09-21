//! What stands in the way, as cells.
//!
//! The senses do not ask what a thing *is*; they ask what a cell does to them. So this
//! is the one place that reads 諸法 and turns a 種 into an effect, and vision, hearing
//! and smell each read a map of cells without learning that a taxonomy exists.
//!
//! An occluder is one cell — the one its position falls in. Anything larger is several
//! occluders, which is how a wall gets built.
//!
//! Sight only, for now. Sound and smell have their own channels declared in
//! `属性.yaml` (音, 匂い) and will want their own maps: an attenuation cost rather than
//! a three-way verdict, since they bend around rather than stop.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::hex::Hex;
use crate::諸法::{実相, 種};

/// The facet axis this reads, and the two values that are not simply "sight passes".
const 視界: &str = "視界";
const 不透明: &str = "不透明";
const 半透明: &str = "半透明";

/// How a cell treats sight.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Opacity {
    /// 透明 — sight passes untouched.
    #[default]
    Clear,
    /// 半透明 — sight passes, at the cost of a band of acuity. Grass hides a rabbit
    /// without hiding a bear, and two thickets cost two bands.
    Obscuring,
    /// 不透明 — sight stops.
    Blocking,
}

impl Opacity {
    /// What a taxonomy node's 視界 facet means here. A kind that declares nothing on the
    /// axis does not impede sight — most things do not.
    fn of(kind: &str, taxonomy: &実相) -> Self {
        match taxonomy.facet_in_group(kind, 視界) {
            Some(値) if 値 == 不透明 => Self::Blocking,
            Some(値) if 値 == 半透明 => Self::Obscuring,
            _ => Self::Clear,
        }
    }
}

/// Which cells impede sight. Rebuilt from the world, so nothing has to remember to
/// register or unregister.
#[derive(Resource, Default)]
pub struct Occluders(HashMap<Hex, Opacity>);

impl Occluders {
    /// A fixed map, for tests and for anything that needs one that is not the world's.
    pub fn from_cells(cells: impl IntoIterator<Item = (Hex, Opacity)>) -> Self {
        Self(cells.into_iter().collect())
    }

    pub fn at(&self, hex: Hex) -> Opacity {
        self.0.get(&hex).copied().unwrap_or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Hex, Opacity)> + '_ {
        self.0.iter().map(|(hex, opacity)| (*hex, *opacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Rebuilt whole each frame rather than maintained. There are a handful of occluders and
/// none of them move; when either changes, this becomes a system that runs on
/// `Changed<GlobalTransform>` and the map stops being rebuilt from nothing.
pub(crate) fn gather_occluders(
    mut occluders: ResMut<Occluders>,
    taxonomy: Res<実相>,
    things: Query<(&GlobalTransform, &種)>,
) {
    occluders.0.clear();
    for (transform, kind) in &things {
        let opacity = Opacity::of(kind.key(), &taxonomy);
        if opacity == Opacity::Clear {
            continue;
        }
        let position = transform.translation();
        let cell = Hex::from_world(Vec2::new(position.x, position.z));
        // Two things in one cell: the stronger wins, which `Ord` on the enum gives.
        let held = occluders.0.entry(cell).or_default();
        *held = (*held).max(opacity);
    }
}

/// Colour of an occluder cell's outline. Neutral, and not either being's hue: what
/// stands in the way belongs to the world, not to whoever is looking at it.
const OCCLUDER_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.9);

/// Outlines every cell that impedes sight, so a shadow can be checked against the thing
/// casting it rather than taken on trust. Weight says which kind it is: heavy stops
/// sight, light costs it a band.
pub(super) fn draw_occluders(
    mut bold: Gizmos<super::BoldStroke>,
    mut faint: Gizmos<super::FaintStroke>,
    occluders: Res<Occluders>,
) {
    for (cell, opacity) in occluders.iter() {
        let corners = cell.corners();
        let outline = corners
            .iter()
            .chain(std::iter::once(&corners[0]))
            .map(|corner| super::ground_point(*corner));
        match opacity {
            Opacity::Blocking => bold.linestrip(outline, OCCLUDER_COLOR),
            Opacity::Obscuring => faint.linestrip(outline, OCCLUDER_COLOR),
            Opacity::Clear => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taxonomy() -> 実相 {
        実相::load()
    }

    /// The point of reading this off the taxonomy: nothing in the scene says "blocks".
    /// A tree blocks because 木 carries 不透明, and grass costs a band because 草 carries
    /// 半透明 — so planting a new kind of thing needs no code here at all.
    #[test]
    fn what_a_kind_does_to_sight_comes_from_its_facet() {
        let tree = taxonomy();
        assert_eq!(Opacity::of("木", &tree), Opacity::Blocking);
        assert_eq!(Opacity::of("草", &tree), Opacity::Obscuring);
    }

    /// Most things do not impede sight, and say nothing on the axis at all.
    #[test]
    fn a_kind_silent_on_the_axis_does_not_impede_sight() {
        let tree = taxonomy();
        assert_eq!(Opacity::of("狐", &tree), Opacity::Clear);
        assert_eq!(Opacity::of("道具", &tree), Opacity::Clear);
        assert_eq!(Opacity::of("未知の何か", &tree), Opacity::Clear);
    }

    fn gathered(things: &[(&str, Vec2)]) -> Occluders {
        let mut app = App::new();
        app.insert_resource(実相::load())
            .init_resource::<Occluders>()
            .add_systems(Update, gather_occluders);
        for (kind, at) in things {
            app.world_mut().spawn((
                種::new(*kind),
                Transform::from_xyz(at.x, 0.0, at.y),
                GlobalTransform::from_xyz(at.x, 0.0, at.y),
            ));
        }
        app.update();
        app.world_mut().remove_resource::<Occluders>().unwrap()
    }

    #[test]
    fn an_occluder_takes_the_cell_it_stands_in() {
        let cell = Hex::new(4, -2);
        let occluders = gathered(&[("木", cell.center())]);
        assert_eq!(occluders.at(cell), Opacity::Blocking);
        assert_eq!(occluders.at(cell.neighbour(0)), Opacity::Clear);
    }

    #[test]
    fn things_that_do_not_impede_sight_are_left_out_entirely() {
        let occluders = gathered(&[("狐", Vec2::ZERO), ("兎", Vec2::new(3.0, 3.0))]);
        assert!(occluders.is_empty(), "{} cells", occluders.len());
    }

    /// A thicket growing where a tree stands does not make the tree see-through.
    #[test]
    fn the_stronger_of_two_in_one_cell_wins() {
        let cell = Hex::new(-1, 5);
        let here = cell.center();
        assert_eq!(
            gathered(&[("草", here), ("木", here)]).at(cell),
            Opacity::Blocking
        );
        assert_eq!(
            gathered(&[("木", here), ("草", here)]).at(cell),
            Opacity::Blocking,
            "order should not matter"
        );
    }
}
