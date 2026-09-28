"""Behaviour trees as files, in BehaviorTree.CPP's XML, so Groot2 can draw and edit them.

An editor edits structure and knows the leaves only by name, so the split is the usual
one: the leaves stay code, each ambition's conditions and acts, and the shape of the tree
is data. Every leaf has an **ID**, its name made an identifier (``looking at us`` is
``looking_at_us``), which is what the file and the editor's palette call it; the node's
``name`` in the file keeps the words, and is what a decision's path shows.

The file is BT.CPP's version 4 format: a ``<root>`` with one ``<BehaviorTree>`` per tree,
``Fallback`` for our selector, ``Sequence`` for our sequence, ``<Condition ID=…/>`` and
``<Action ID=…/>`` for the leaves, and a ``<TreeNodesModel>`` listing every leaf, which is
Groot2's palette. ``export`` writes every ambition's tree to ``ai/midbrain/trees/``, and a
test fails when the checked-in files fall behind the code; ``load`` builds a tree back from
a file with an ambition's leaves.
"""

from __future__ import annotations

import re
import xml.etree.ElementTree as ET
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from midbrain.behaviour import Act, Condition, Node, Selector, Sequence

TREES = Path(__file__).resolve().parents[2] / "trees"
"""Where the tree files live: ``ai/midbrain/trees/<ambition>.xml``."""


def identifier(name: str) -> str:
    """A node's name as an identifier: ``beside it, facing it`` → ``beside_it_facing_it``."""
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


class UnknownLeaf(KeyError):
    """A file names a leaf the ambition does not have."""


class Malformed(ValueError):
    """A file that is not a tree as this module writes them."""


@dataclass
class Leaves:
    """An ambition's leaves by ID: what a file may refer to."""

    conditions: dict[str, Callable[[Any], bool]] = field(default_factory=dict)
    acts: dict[str, Callable[[Any], Any]] = field(default_factory=dict)

    def condition(self, id: str, name: str) -> Condition:
        if id not in self.conditions:
            raise UnknownLeaf(f"no condition {id!r}; there are {sorted(self.conditions)}")
        return Condition(name, self.conditions[id])

    def act(self, id: str, name: str) -> Act:
        if id not in self.acts:
            raise UnknownLeaf(f"no act {id!r}; there are {sorted(self.acts)}")
        return Act(name, self.acts[id])


def leaves_of(tree: Node) -> Leaves:
    """Every leaf in a tree, by the ID its name makes."""
    leaves = Leaves()
    for node in walk(tree):
        if isinstance(node, Condition):
            leaves.conditions[identifier(node.name)] = node.test
        elif isinstance(node, Act):
            leaves.acts[identifier(node.name)] = node.want
    return leaves


def walk(node: Node):
    """The node and everything under it, depth first."""
    yield node
    for child in getattr(node, "children", ()):
        yield from walk(child)


# ---- out ----------------------------------------------------------------------------


def to_xml(tree: Node) -> str:
    """One tree as a BT.CPP file, its root named after the tree."""
    root = ET.Element("root", BTCPP_format="4", main_tree_to_execute=tree.name)
    behaviour = ET.SubElement(root, "BehaviorTree", ID=tree.name)
    behaviour.append(_element(tree))
    palette = ET.SubElement(root, "TreeNodesModel")
    leaves = leaves_of(tree)
    for id in sorted(leaves.conditions):
        ET.SubElement(palette, "Condition", ID=id)
    for id in sorted(leaves.acts):
        ET.SubElement(palette, "Action", ID=id)
    ET.indent(root, space="  ")
    return ET.tostring(root, encoding="unicode") + "\n"


def _element(node: Node) -> ET.Element:
    match node:
        case Selector():
            element = ET.Element("Fallback", name=node.name)
        case Sequence():
            element = ET.Element("Sequence", name=node.name)
        case Condition():
            return ET.Element("Condition", ID=identifier(node.name), name=node.name)
        case Act():
            return ET.Element("Action", ID=identifier(node.name), name=node.name)
        case _:
            raise Malformed(f"no XML for a {type(node).__name__}")
    for child in node.children:
        element.append(_element(child))
    return element


# ---- in -----------------------------------------------------------------------------


def from_xml(text: str, leaves: Leaves) -> Node:
    """The tree a file describes, its leaves taken from an ambition's."""
    root = ET.fromstring(text)
    if root.tag != "root":
        raise Malformed(f"expected <root>, found <{root.tag}>")
    trees = root.findall("BehaviorTree")
    if len(trees) != 1:
        raise Malformed(f"expected one <BehaviorTree>, found {len(trees)}")
    children = list(trees[0])
    if len(children) != 1:
        raise Malformed(f"a <BehaviorTree> has one node under it, found {len(children)}")
    return _node(children[0], leaves)


def _node(element: ET.Element, leaves: Leaves) -> Node:
    name = element.get("name") or element.get("ID") or element.tag
    match element.tag:
        case "Fallback":
            return Selector(name, tuple(_node(child, leaves) for child in element))
        case "Sequence":
            return Sequence(name, tuple(_node(child, leaves) for child in element))
        case "Condition":
            return leaves.condition(element.get("ID", ""), name)
        case "Action":
            return leaves.act(element.get("ID", ""), name)
        case other:
            raise Malformed(f"no node for <{other}>")


def load(name: str, leaves: Leaves) -> Node:
    """The tree in ``trees/<name>.xml``."""
    return from_xml((TREES / f"{name}.xml").read_text(), leaves)


# ---- the files ----------------------------------------------------------------------


def exports() -> dict[str, str]:
    """Every ambition with a tree, and its file's text, as the code has it now."""
    from midbrain.ambitions import REPERTOIRE

    files = {}
    for ambitions in REPERTOIRE.values():
        for ambition in ambitions:
            tree = getattr(ambition, "tree", None)
            if tree is not None:
                files[ambition.name] = to_xml(tree)
    return files


def export(into: Path = TREES) -> list[Path]:
    """Writes every tree file; ``uv run trees``."""
    into.mkdir(parents=True, exist_ok=True)
    written = []
    for name, text in exports().items():
        path = into / f"{name}.xml"
        path.write_text(text)
        written.append(path)
    return written


def main() -> None:
    for path in export():
        print(f"wrote {path}")
