"""A small behaviour tree: the shape an ambition's decisions take.

Halo's layering: utility picks *which* ambition runs, and the ambition runs a tree to say
*what* it wants this round. A tree here is four kinds of node and one verb, ``tick``, which
takes whatever the tree reads (the believed world and the body's snapshot, in practice) and
answers with a ``Result``: did the branch succeed, what intent it wants (if any), and the
names of the nodes on the winning path, for a reader watching the mind think.

- ``Condition``: a test; succeeds or fails, wants nothing.
- ``Act``: a want; succeeds with an intent, or with none, which is "hold as you are".
- ``Selector``: its children in order, the first to succeed wins.
- ``Sequence``: its children in order, all must succeed; the last one's want is the result.

Nothing runs across rounds: a tree is ticked afresh each time from the root. Whatever
memory an ambition needs lives in the believed world, not in the tree.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, Protocol

from midbrain import murabito_pb2 as pb


@dataclass(frozen=True)
class Result:
    ok: bool
    intent: pb.Intent | None = None
    path: tuple[str, ...] = ()
    """The names of the nodes on the path that produced this result, root first."""

    def under(self, name: str) -> Result:
        """The same result, seen from the parent node ``name``."""
        return Result(self.ok, self.intent, (name, *self.path))


FAILURE = Result(False)


class Node(Protocol):
    name: str

    def tick(self, ctx: Any) -> Result: ...


@dataclass(frozen=True)
class Condition:
    name: str
    test: Callable[[Any], bool]

    def tick(self, ctx: Any) -> Result:
        return Result(True, None, (self.name,)) if self.test(ctx) else FAILURE


@dataclass(frozen=True)
class Act:
    name: str
    want: Callable[[Any], pb.Intent | None]

    def tick(self, ctx: Any) -> Result:
        return Result(True, self.want(ctx), (self.name,))


@dataclass(frozen=True)
class Selector:
    name: str
    children: tuple[Node, ...]

    def tick(self, ctx: Any) -> Result:
        for child in self.children:
            result = child.tick(ctx)
            if result.ok:
                return result.under(self.name)
        return FAILURE


@dataclass(frozen=True)
class Sequence:
    name: str
    children: tuple[Node, ...]

    def tick(self, ctx: Any) -> Result:
        result = Result(True)
        for child in self.children:
            result = child.tick(ctx)
            if not result.ok:
                return FAILURE
        return result.under(self.name)
