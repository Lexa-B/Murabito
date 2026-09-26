//! Conversion at the edge: the brainstem's types to the contract's messages and back.
//!
//! Outward, everything converts: a snapshot is plain data with nothing to refuse.
//! Inward, an intent can be malformed, since a message's fields are all optional on the
//! wire, and a malformed one is refused with a word on what was missing, never guessed.

use murabito_actions::Action;
use murabito_brainstem::{Doing, InView, Intent, Outcome, Short, Snapshot, Sustained};
use murabito_hexcoords::{Direction, Offset, VoxelCoord};
use murabito_identity::ThingId;
use murabito_vision::Acuity;

use crate::proto;

/// What was wrong with an inbound message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Malformed(pub &'static str);

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "malformed: {}", self.0)
    }
}

impl std::error::Error for Malformed {}

// ---- out -------------------------------------------------------------------------

/// A direction's number on the wire is its index in memory.
fn direction_out(direction: Direction) -> i32 {
    i32::from(direction.index())
}

fn acuity_out(acuity: Acuity) -> i32 {
    match acuity {
        Acuity::Near => proto::Acuity::Near as i32,
        Acuity::Mid => proto::Acuity::Mid as i32,
        Acuity::Far => proto::Acuity::Far as i32,
    }
}

impl From<VoxelCoord> for proto::Voxel {
    fn from(voxel: VoxelCoord) -> Self {
        Self {
            q: voxel.q(),
            r: voxel.r(),
            layer: voxel.layer(),
        }
    }
}

impl From<Offset> for proto::Offset {
    fn from(offset: Offset) -> Self {
        Self {
            dq: offset.dq(),
            dr: offset.dr(),
            dlayer: offset.dlayer(),
        }
    }
}

impl From<Short> for proto::Short {
    fn from(short: Short) -> Self {
        use proto::short::Kind;
        let kind = match short {
            Short::Stop => Kind::Stop(proto::Stop {}),
            Short::Face(direction) => Kind::Face(direction_out(direction)),
            Short::FaceThing(id) => Kind::FaceThing(id.number()),
            Short::Step(direction) => Kind::Step(direction_out(direction)),
        };
        Self { kind: Some(kind) }
    }
}

impl From<Sustained> for proto::Sustained {
    fn from(sustained: Sustained) -> Self {
        use proto::sustained::Kind;
        let kind = match sustained {
            Sustained::GoTo(voxel) => Kind::GoTo(voxel.into()),
        };
        Self { kind: Some(kind) }
    }
}

impl From<Intent> for proto::Intent {
    fn from(intent: Intent) -> Self {
        use proto::intent::Kind;
        let kind = match intent {
            Intent::Short(short) => Kind::Short(short.into()),
            Intent::Sustained(sustained) => Kind::Sustained(sustained.into()),
        };
        Self { kind: Some(kind) }
    }
}

impl From<Action> for proto::Action {
    fn from(action: Action) -> Self {
        use proto::action::Kind;
        let kind = match action {
            Action::Go(direction) => Kind::Go(direction_out(direction)),
            Action::Face(direction) => Kind::Face(direction_out(direction)),
        };
        Self { kind: Some(kind) }
    }
}

impl From<Outcome> for proto::Outcome {
    fn from(outcome: Outcome) -> Self {
        use proto::outcome::Kind;
        let kind = match outcome {
            Outcome::Idle => Kind::Idle(proto::Idle {}),
            Outcome::Done => Kind::Done(proto::Done {}),
            Outcome::Stopped => Kind::Stopped(proto::Stopped {}),
            Outcome::Superseded => Kind::Superseded(proto::Superseded {}),
            Outcome::Cancelled(reflex) => Kind::Cancelled(reflex.to_owned()),
            Outcome::Lost(id) => Kind::Lost(id.number()),
        };
        Self { kind: Some(kind) }
    }
}

impl From<Doing> for proto::Doing {
    fn from(doing: Doing) -> Self {
        Self {
            intent: Some(doing.intent().into()),
            since: doing.since(),
        }
    }
}

impl From<InView> for proto::InView {
    fn from(seen: InView) -> Self {
        Self {
            id: seen.id.number(),
            kind: seen.kind.map(|kind| kind.path().to_owned()),
            offset: Some(seen.offset.into()),
            distance: seen.distance,
            acuity: acuity_out(seen.acuity),
        }
    }
}

impl From<Snapshot> for proto::Snapshot {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            id: snapshot.id.number(),
            kind: snapshot.kind.path().to_owned(),
            tick: snapshot.tick,
            position: Some(snapshot.position.into()),
            facing: direction_out(snapshot.facing),
            in_view: snapshot.in_view.into_iter().map(Into::into).collect(),
            doing: snapshot.doing.map(Into::into),
            queue: snapshot.queue.into_iter().map(Into::into).collect(),
            in_flight: snapshot.in_flight,
            previous_outcome: Some(snapshot.previous_outcome.into()),
        }
    }
}

/// Every body's snapshot, as one message.
pub fn snapshots(bodies: impl IntoIterator<Item = Snapshot>) -> proto::Snapshots {
    proto::Snapshots {
        bodies: bodies.into_iter().map(Into::into).collect(),
    }
}

// ---- in --------------------------------------------------------------------------

fn direction_in(number: i32) -> Result<Direction, Malformed> {
    usize::try_from(number)
        .ok()
        .and_then(|index| Direction::ALL.get(index).copied())
        .ok_or(Malformed("a direction must be 0 to 11"))
}

impl TryFrom<proto::Voxel> for VoxelCoord {
    type Error = Malformed;

    /// Axial on the wire, so always on the plane; only overflow could refuse it.
    fn try_from(voxel: proto::Voxel) -> Result<Self, Malformed> {
        let s = voxel
            .q
            .checked_add(voxel.r)
            .and_then(i32::checked_neg)
            .ok_or(Malformed("a voxel too far out to name"))?;
        VoxelCoord::new(voxel.q, voxel.r, s, voxel.layer)
            .map_err(|_| Malformed("a voxel off the plane"))
    }
}

impl TryFrom<proto::Short> for Short {
    type Error = Malformed;

    fn try_from(short: proto::Short) -> Result<Self, Malformed> {
        use proto::short::Kind;
        Ok(match short.kind.ok_or(Malformed("a short with no kind"))? {
            Kind::Stop(_) => Short::Stop,
            Kind::Face(direction) => Short::Face(direction_in(direction)?),
            Kind::FaceThing(id) => Short::FaceThing(ThingId::restored(id)),
            Kind::Step(direction) => Short::Step(direction_in(direction)?),
        })
    }
}

impl TryFrom<proto::Sustained> for Sustained {
    type Error = Malformed;

    fn try_from(sustained: proto::Sustained) -> Result<Self, Malformed> {
        use proto::sustained::Kind;
        Ok(
            match sustained
                .kind
                .ok_or(Malformed("a sustained intent with no kind"))?
            {
                Kind::GoTo(voxel) => Sustained::GoTo(voxel.try_into()?),
            },
        )
    }
}

impl TryFrom<proto::Intent> for Intent {
    type Error = Malformed;

    fn try_from(intent: proto::Intent) -> Result<Self, Malformed> {
        use proto::intent::Kind;
        Ok(
            match intent.kind.ok_or(Malformed("an intent with no kind"))? {
                Kind::Short(short) => Intent::Short(short.try_into()?),
                Kind::Sustained(sustained) => Intent::Sustained(sustained.try_into()?),
            },
        )
    }
}

/// An order as the brainstem takes it: who, and what.
pub fn order_in(order: proto::Order) -> Result<(ThingId, Intent), Malformed> {
    let intent = order.intent.ok_or(Malformed("an order with no intent"))?;
    Ok((ThingId::restored(order.id), intent.try_into()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use murabito_identity::Kind;

    fn voxel(q: i32, r: i32) -> VoxelCoord {
        VoxelCoord::new(q, r, -q - r, 0).unwrap()
    }

    #[test]
    fn a_direction_crosses_the_wire_as_its_index_and_comes_back() {
        for direction in Direction::ALL {
            let number = direction_out(direction);
            assert_eq!(number, i32::from(direction.index()));
            assert_eq!(direction_in(number), Ok(direction));
        }
        assert!(direction_in(12).is_err());
        assert!(direction_in(-1).is_err());
    }

    #[test]
    fn an_intent_goes_out_and_comes_back_the_same() {
        let intents = [
            Intent::Short(Short::Stop),
            Intent::Short(Short::Face(Direction::N)),
            Intent::Short(Short::FaceThing(ThingId::restored(7))),
            Intent::Short(Short::Step(Direction::WSW)),
            Intent::Sustained(Sustained::GoTo(voxel(3, -5))),
        ];
        for intent in intents {
            let message: proto::Intent = intent.into();
            assert_eq!(Intent::try_from(message), Ok(intent));
        }
    }

    #[test]
    fn a_message_missing_its_kind_is_refused_and_says_what_was_missing() {
        assert_eq!(
            Intent::try_from(proto::Intent { kind: None }),
            Err(Malformed("an intent with no kind"))
        );
        assert_eq!(
            order_in(proto::Order {
                id: 3,
                intent: None
            }),
            Err(Malformed("an order with no intent"))
        );
        let bad_direction = proto::Short {
            kind: Some(proto::short::Kind::Face(40)),
        };
        assert_eq!(
            Short::try_from(bad_direction),
            Err(Malformed("a direction must be 0 to 11"))
        );
    }

    #[test]
    fn an_order_names_the_body_by_its_number() {
        let order = proto::Order {
            id: 42,
            intent: Some(Intent::Short(Short::Stop).into()),
        };
        assert_eq!(
            order_in(order),
            Ok((ThingId::restored(42), Intent::Short(Short::Stop)))
        );
    }

    #[test]
    fn a_snapshot_goes_out_field_for_field() {
        let snapshot = Snapshot {
            id: ThingId::restored(2),
            kind: Kind::at("test::fox"),
            tick: 9,
            position: voxel(-8, 0),
            facing: Direction::ESE,
            in_view: vec![InView {
                id: ThingId::restored(3),
                kind: None,
                offset: voxel(2, -1) - voxel(0, 0),
                distance: 2,
                acuity: Acuity::Mid,
            }],
            doing: None,
            queue: vec![Action::Go(Direction::E), Action::Face(Direction::N)],
            in_flight: Some(0.25),
            previous_outcome: Outcome::Cancelled("startle_face_apparition"),
        };
        let message: proto::Snapshot = snapshot.into();
        assert_eq!(message.id, 2);
        assert_eq!(message.kind, "test::fox");
        assert_eq!(message.tick, 9);
        assert_eq!(
            message.position,
            Some(proto::Voxel {
                q: -8,
                r: 0,
                layer: 0
            })
        );
        assert_eq!(message.facing, 11);
        assert_eq!(message.in_view.len(), 1);
        assert_eq!(message.in_view[0].kind, None);
        assert_eq!(
            message.in_view[0].offset,
            Some(proto::Offset {
                dq: 2,
                dr: -1,
                dlayer: 0
            })
        );
        assert_eq!(message.in_view[0].acuity, proto::Acuity::Mid as i32);
        assert_eq!(message.doing, None);
        assert_eq!(message.queue.len(), 2);
        assert_eq!(message.queue[0].kind, Some(proto::action::Kind::Go(0)));
        assert_eq!(message.in_flight, Some(0.25));
        assert_eq!(
            message.previous_outcome.unwrap().kind,
            Some(proto::outcome::Kind::Cancelled(
                "startle_face_apparition".to_owned()
            ))
        );
    }
}
