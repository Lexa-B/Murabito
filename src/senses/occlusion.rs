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

/// The other axis an occluder is read on, and its values tallest first.
const 高さ: &str = "高さ";
const 背丈: &str = "背丈";
const 腰丈: &str = "腰丈";

/// How many height classes there are. Named so the arrays that carry one entry per
/// class say what they are counting.
pub const HEIGHTS: usize = 3;

/// How tall a thing is — and so, when it stands in the way, how much of the world it
/// stands in the way of.
///
/// The same number on both sides: what a thing hides and what can hide it are one
/// property. An occluder affects whatever is no taller than itself, which is the whole
/// rule, and `Ord` is what states it.
///
/// Defaults to `Full`, which is the cautious reading in both directions: an occluder
/// that declares no height stands in everything's way, and a target that declares none
/// is the hardest to hide.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Height {
    /// 膝丈 — knee-high. Grass, and most of what walks on four legs.
    Knee,
    /// 腰丈 — waist-high. A thicket, a hedge, a wolf.
    Waist,
    /// 背丈 — a standing person.
    #[default]
    Full,
}

impl Height {
    /// Index into a per-class array. Shortest first, so `..=index()` is "everything this
    /// tall and under" — the set a thing of this height stands in the way of.
    pub fn index(self) -> usize {
        match self {
            Self::Knee => 0,
            Self::Waist => 1,
            Self::Full => 2,
        }
    }

    fn of(kind: &str, taxonomy: &実相) -> Self {
        match taxonomy.facet_in_group(kind, 高さ) {
            Some(値) if 値 == 背丈 => Self::Full,
            Some(値) if 値 == 腰丈 => Self::Waist,
            Some(_) => Self::Knee,
            None => Self::default(),
        }
    }
}

/// Stamped onto anything carrying a 種, so the senses can ask how tall something is
/// without learning that a taxonomy exists — the same seam `Occluders` is.
pub(crate) fn read_heights(
    taxonomy: Res<実相>,
    mut commands: Commands,
    things: Query<(Entity, &種), Changed<種>>,
) {
    for (entity, kind) in &things {
        commands
            .entity(entity)
            .insert(Height::of(kind.key(), &taxonomy));
    }
}

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
pub struct Occluders(HashMap<Hex, (Opacity, Height)>);

impl Occluders {
    /// A fixed map, for tests and for anything that needs one that is not the world's.
    pub fn from_cells(cells: impl IntoIterator<Item = (Hex, Opacity, Height)>) -> Self {
        Self(
            cells
                .into_iter()
                .map(|(hex, opacity, height)| (hex, (opacity, height)))
                .collect(),
        )
    }

    /// What stands in this cell: what it does to sight, and how tall it is.
    pub fn at(&self, hex: Hex) -> (Opacity, Height) {
        self.0
            .get(&hex)
            .copied()
            .unwrap_or((Opacity::Clear, Height::default()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (Hex, Opacity, Height)> + '_ {
        self.0
            .iter()
            .map(|(hex, (opacity, height))| (*hex, *opacity, *height))
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
        let height = Height::of(kind.key(), &taxonomy);
        // Two things in one cell: the stronger and the taller of them, which `Ord` on
        // each gives. A tree beside a tuft of grass is a tree.
        let held = occluders
            .0
            .entry(cell)
            .or_insert((Opacity::Clear, Height::Knee));
        held.0 = held.0.max(opacity);
        held.1 = held.1.max(height);
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
    for (cell, opacity, _) in occluders.iter() {
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
        assert_eq!(occluders.at(cell), (Opacity::Blocking, Height::Full));
        assert_eq!(
            occluders.at(cell.neighbour(0)),
            (Opacity::Clear, Height::Full)
        );
    }

    #[test]
    fn things_that_do_not_impede_sight_are_left_out_entirely() {
        let occluders = gathered(&[("狐", Vec2::ZERO), ("兎", Vec2::new(3.0, 3.0))]);
        assert!(occluders.is_empty(), "{} cells", occluders.len());
    }

    /// Grass growing where a tree stands does not make the tree see-through, nor cut it
    /// down to knee height.
    #[test]
    fn the_stronger_and_taller_of_two_in_one_cell_wins() {
        let cell = Hex::new(-1, 5);
        let here = cell.center();
        for order in [["草", "木"], ["木", "草"]] {
            let gathered = gathered(&[(order[0], here), (order[1], here)]);
            assert_eq!(
                gathered.at(cell),
                (Opacity::Blocking, Height::Full),
                "order should not matter: {order:?}"
            );
        }
    }

    /// Height is read off the same axis for anything that carries one, occluder or not.
    #[test]
    fn a_kind_is_as_tall_as_its_facet_says() {
        let tree = 実相::load();
        assert_eq!(Height::of("木", &tree), Height::Full);
        assert_eq!(Height::of("茂み", &tree), Height::Waist);
        assert_eq!(Height::of("草", &tree), Height::Knee);
        assert_eq!(Height::of("兎", &tree), Height::Knee);
        assert_eq!(Height::of("人間", &tree), Height::Full);
        // Nothing declared: treated as standing in everything's way, and as the hardest
        // thing to hide.
        assert_eq!(Height::of("道具", &tree), Height::Full);
    }
}
