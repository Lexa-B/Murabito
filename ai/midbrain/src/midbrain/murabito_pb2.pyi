from google.protobuf.internal import containers as _containers
from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from collections.abc import Iterable as _Iterable, Mapping as _Mapping
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Direction(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    E: _ClassVar[Direction]
    ENE: _ClassVar[Direction]
    NNE: _ClassVar[Direction]
    N: _ClassVar[Direction]
    NNW: _ClassVar[Direction]
    WNW: _ClassVar[Direction]
    W: _ClassVar[Direction]
    WSW: _ClassVar[Direction]
    SSW: _ClassVar[Direction]
    S: _ClassVar[Direction]
    SSE: _ClassVar[Direction]
    ESE: _ClassVar[Direction]

class Acuity(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    NEAR: _ClassVar[Acuity]
    MID: _ClassVar[Acuity]
    FAR: _ClassVar[Acuity]
E: Direction
ENE: Direction
NNE: Direction
N: Direction
NNW: Direction
WNW: Direction
W: Direction
WSW: Direction
SSW: Direction
S: Direction
SSE: Direction
ESE: Direction
NEAR: Acuity
MID: Acuity
FAR: Acuity

class Request(_message.Message):
    __slots__ = ("snapshots", "order")
    SNAPSHOTS_FIELD_NUMBER: _ClassVar[int]
    ORDER_FIELD_NUMBER: _ClassVar[int]
    snapshots: SnapshotsRequest
    order: Order
    def __init__(self, snapshots: _Optional[_Union[SnapshotsRequest, _Mapping]] = ..., order: _Optional[_Union[Order, _Mapping]] = ...) -> None: ...

class SnapshotsRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Order(_message.Message):
    __slots__ = ("id", "intent")
    ID_FIELD_NUMBER: _ClassVar[int]
    INTENT_FIELD_NUMBER: _ClassVar[int]
    id: int
    intent: Intent
    def __init__(self, id: _Optional[int] = ..., intent: _Optional[_Union[Intent, _Mapping]] = ...) -> None: ...

class Snapshots(_message.Message):
    __slots__ = ("bodies",)
    BODIES_FIELD_NUMBER: _ClassVar[int]
    bodies: _containers.RepeatedCompositeFieldContainer[Snapshot]
    def __init__(self, bodies: _Optional[_Iterable[_Union[Snapshot, _Mapping]]] = ...) -> None: ...

class Snapshot(_message.Message):
    __slots__ = ("id", "kind", "tick", "position", "facing", "in_view", "doing", "queue", "in_flight", "previous", "underway")
    ID_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    TICK_FIELD_NUMBER: _ClassVar[int]
    POSITION_FIELD_NUMBER: _ClassVar[int]
    FACING_FIELD_NUMBER: _ClassVar[int]
    IN_VIEW_FIELD_NUMBER: _ClassVar[int]
    DOING_FIELD_NUMBER: _ClassVar[int]
    QUEUE_FIELD_NUMBER: _ClassVar[int]
    IN_FLIGHT_FIELD_NUMBER: _ClassVar[int]
    PREVIOUS_FIELD_NUMBER: _ClassVar[int]
    UNDERWAY_FIELD_NUMBER: _ClassVar[int]
    id: int
    kind: str
    tick: int
    position: Voxel
    facing: Direction
    in_view: _containers.RepeatedCompositeFieldContainer[InView]
    doing: Doing
    queue: _containers.RepeatedCompositeFieldContainer[Action]
    in_flight: float
    previous: Previous
    underway: Action
    def __init__(self, id: _Optional[int] = ..., kind: _Optional[str] = ..., tick: _Optional[int] = ..., position: _Optional[_Union[Voxel, _Mapping]] = ..., facing: _Optional[_Union[Direction, str]] = ..., in_view: _Optional[_Iterable[_Union[InView, _Mapping]]] = ..., doing: _Optional[_Union[Doing, _Mapping]] = ..., queue: _Optional[_Iterable[_Union[Action, _Mapping]]] = ..., in_flight: _Optional[float] = ..., previous: _Optional[_Union[Previous, _Mapping]] = ..., underway: _Optional[_Union[Action, _Mapping]] = ...) -> None: ...

class InView(_message.Message):
    __slots__ = ("id", "kind", "offset", "distance", "acuity", "facing")
    ID_FIELD_NUMBER: _ClassVar[int]
    KIND_FIELD_NUMBER: _ClassVar[int]
    OFFSET_FIELD_NUMBER: _ClassVar[int]
    DISTANCE_FIELD_NUMBER: _ClassVar[int]
    ACUITY_FIELD_NUMBER: _ClassVar[int]
    FACING_FIELD_NUMBER: _ClassVar[int]
    id: int
    kind: str
    offset: Offset
    distance: int
    acuity: Acuity
    facing: Direction
    def __init__(self, id: _Optional[int] = ..., kind: _Optional[str] = ..., offset: _Optional[_Union[Offset, _Mapping]] = ..., distance: _Optional[int] = ..., acuity: _Optional[_Union[Acuity, str]] = ..., facing: _Optional[_Union[Direction, str]] = ...) -> None: ...

class Doing(_message.Message):
    __slots__ = ("intent", "since")
    INTENT_FIELD_NUMBER: _ClassVar[int]
    SINCE_FIELD_NUMBER: _ClassVar[int]
    intent: Intent
    since: int
    def __init__(self, intent: _Optional[_Union[Intent, _Mapping]] = ..., since: _Optional[int] = ...) -> None: ...

class Previous(_message.Message):
    __slots__ = ("intent", "outcome")
    INTENT_FIELD_NUMBER: _ClassVar[int]
    OUTCOME_FIELD_NUMBER: _ClassVar[int]
    intent: Intent
    outcome: Outcome
    def __init__(self, intent: _Optional[_Union[Intent, _Mapping]] = ..., outcome: _Optional[_Union[Outcome, _Mapping]] = ...) -> None: ...

class Intent(_message.Message):
    __slots__ = ("short", "path")
    SHORT_FIELD_NUMBER: _ClassVar[int]
    PATH_FIELD_NUMBER: _ClassVar[int]
    short: Short
    path: Path
    def __init__(self, short: _Optional[_Union[Short, _Mapping]] = ..., path: _Optional[_Union[Path, _Mapping]] = ...) -> None: ...

class Short(_message.Message):
    __slots__ = ("stop", "face", "face_thing", "walk", "jog", "sprint", "sneak", "sidestep", "backstep", "recoil", "lunge", "bite")
    STOP_FIELD_NUMBER: _ClassVar[int]
    FACE_FIELD_NUMBER: _ClassVar[int]
    FACE_THING_FIELD_NUMBER: _ClassVar[int]
    WALK_FIELD_NUMBER: _ClassVar[int]
    JOG_FIELD_NUMBER: _ClassVar[int]
    SPRINT_FIELD_NUMBER: _ClassVar[int]
    SNEAK_FIELD_NUMBER: _ClassVar[int]
    SIDESTEP_FIELD_NUMBER: _ClassVar[int]
    BACKSTEP_FIELD_NUMBER: _ClassVar[int]
    RECOIL_FIELD_NUMBER: _ClassVar[int]
    LUNGE_FIELD_NUMBER: _ClassVar[int]
    BITE_FIELD_NUMBER: _ClassVar[int]
    stop: Stop
    face: Direction
    face_thing: int
    walk: Direction
    jog: Direction
    sprint: Direction
    sneak: Direction
    sidestep: Direction
    backstep: Direction
    recoil: Direction
    lunge: Direction
    bite: Bite
    def __init__(self, stop: _Optional[_Union[Stop, _Mapping]] = ..., face: _Optional[_Union[Direction, str]] = ..., face_thing: _Optional[int] = ..., walk: _Optional[_Union[Direction, str]] = ..., jog: _Optional[_Union[Direction, str]] = ..., sprint: _Optional[_Union[Direction, str]] = ..., sneak: _Optional[_Union[Direction, str]] = ..., sidestep: _Optional[_Union[Direction, str]] = ..., backstep: _Optional[_Union[Direction, str]] = ..., recoil: _Optional[_Union[Direction, str]] = ..., lunge: _Optional[_Union[Direction, str]] = ..., bite: _Optional[_Union[Bite, _Mapping]] = ...) -> None: ...

class Stop(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Path(_message.Message):
    __slots__ = ("keep", "steps")
    KEEP_FIELD_NUMBER: _ClassVar[int]
    STEPS_FIELD_NUMBER: _ClassVar[int]
    keep: int
    steps: _containers.RepeatedCompositeFieldContainer[Action]
    def __init__(self, keep: _Optional[int] = ..., steps: _Optional[_Iterable[_Union[Action, _Mapping]]] = ...) -> None: ...

class Action(_message.Message):
    __slots__ = ("walk", "jog", "sprint", "sneak", "face", "sidestep", "backstep", "recoil", "lunge", "bite")
    WALK_FIELD_NUMBER: _ClassVar[int]
    JOG_FIELD_NUMBER: _ClassVar[int]
    SPRINT_FIELD_NUMBER: _ClassVar[int]
    SNEAK_FIELD_NUMBER: _ClassVar[int]
    FACE_FIELD_NUMBER: _ClassVar[int]
    SIDESTEP_FIELD_NUMBER: _ClassVar[int]
    BACKSTEP_FIELD_NUMBER: _ClassVar[int]
    RECOIL_FIELD_NUMBER: _ClassVar[int]
    LUNGE_FIELD_NUMBER: _ClassVar[int]
    BITE_FIELD_NUMBER: _ClassVar[int]
    walk: Direction
    jog: Direction
    sprint: Direction
    sneak: Direction
    face: Direction
    sidestep: Direction
    backstep: Direction
    recoil: Direction
    lunge: Direction
    bite: Bite
    def __init__(self, walk: _Optional[_Union[Direction, str]] = ..., jog: _Optional[_Union[Direction, str]] = ..., sprint: _Optional[_Union[Direction, str]] = ..., sneak: _Optional[_Union[Direction, str]] = ..., face: _Optional[_Union[Direction, str]] = ..., sidestep: _Optional[_Union[Direction, str]] = ..., backstep: _Optional[_Union[Direction, str]] = ..., recoil: _Optional[_Union[Direction, str]] = ..., lunge: _Optional[_Union[Direction, str]] = ..., bite: _Optional[_Union[Bite, _Mapping]] = ...) -> None: ...

class Bite(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Outcome(_message.Message):
    __slots__ = ("done", "stopped", "superseded", "cancelled", "lost", "refused")
    DONE_FIELD_NUMBER: _ClassVar[int]
    STOPPED_FIELD_NUMBER: _ClassVar[int]
    SUPERSEDED_FIELD_NUMBER: _ClassVar[int]
    CANCELLED_FIELD_NUMBER: _ClassVar[int]
    LOST_FIELD_NUMBER: _ClassVar[int]
    REFUSED_FIELD_NUMBER: _ClassVar[int]
    done: Done
    stopped: Stopped
    superseded: Superseded
    cancelled: str
    lost: int
    refused: str
    def __init__(self, done: _Optional[_Union[Done, _Mapping]] = ..., stopped: _Optional[_Union[Stopped, _Mapping]] = ..., superseded: _Optional[_Union[Superseded, _Mapping]] = ..., cancelled: _Optional[str] = ..., lost: _Optional[int] = ..., refused: _Optional[str] = ...) -> None: ...

class Done(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Stopped(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Superseded(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class Voxel(_message.Message):
    __slots__ = ("q", "r", "layer")
    Q_FIELD_NUMBER: _ClassVar[int]
    R_FIELD_NUMBER: _ClassVar[int]
    LAYER_FIELD_NUMBER: _ClassVar[int]
    q: int
    r: int
    layer: int
    def __init__(self, q: _Optional[int] = ..., r: _Optional[int] = ..., layer: _Optional[int] = ...) -> None: ...

class Offset(_message.Message):
    __slots__ = ("dq", "dr", "dlayer")
    DQ_FIELD_NUMBER: _ClassVar[int]
    DR_FIELD_NUMBER: _ClassVar[int]
    DLAYER_FIELD_NUMBER: _ClassVar[int]
    dq: int
    dr: int
    dlayer: int
    def __init__(self, dq: _Optional[int] = ..., dr: _Optional[int] = ..., dlayer: _Optional[int] = ...) -> None: ...
