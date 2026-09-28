"""Trees as files: out to BT.CPP's XML and back, and the checked-in files current."""

from __future__ import annotations

import xml.etree.ElementTree as ET

import pytest

from midbrain.ambitions import HARE, Stalk, Wander
from midbrain.behaviour import Act, Condition, Selector, Sequence
from midbrain.trees import TREES, Malformed, UnknownLeaf, exports, from_xml, identifier, leaves_of, to_xml
from test_ambitions import fox_believing


def test_a_name_makes_an_identifier() -> None:
    assert identifier("looking at us") == "looking_at_us"
    assert identifier("beside it, facing it") == "beside_it_facing_it"
    assert identifier("Set Up!") == "set_up"


def test_the_stalks_tree_goes_out_as_bt_cpp_xml() -> None:
    root = ET.fromstring(to_xml(Stalk(prey=HARE).tree))
    assert root.tag == "root" and root.get("BTCPP_format") == "4" and root.get("main_tree_to_execute") == "stalk"
    tree = root.find("BehaviorTree")
    assert tree.get("ID") == "stalk"
    top = tree[0]
    assert top.tag == "Fallback" and top.get("name") == "stalk"
    assert [child.get("name") for child in top] == ["freeze", "bite", "pounce", "set up", "check", "circle", "approach", "watch"]
    freeze = top[0]
    assert freeze.tag == "Sequence"
    assert (freeze[0].tag, freeze[0].get("ID"), freeze[0].get("name")) == ("Condition", "looking_at_us", "looking at us")
    assert (freeze[1].tag, freeze[1].get("ID")) == ("Action", "stop")
    palette = root.find("TreeNodesModel")
    assert {leaf.get("ID") for leaf in palette.findall("Condition")} >= {"looking_at_us", "unseen_too_long", "off_the_pounce_line"}
    assert {leaf.get("ID") for leaf in palette.findall("Action")} >= {"stop", "lunge", "sidestep", "watch"}


def test_a_tree_comes_back_from_its_file_the_same_and_decides_the_same() -> None:
    # One build of each tree: the property builds afresh, and a fresh lambda never equals
    # the last one, so the comparison is against the very tree that was exported.
    stalk = Stalk(prey=HARE).tree
    loaded = from_xml(to_xml(stalk), leaves_of(stalk))
    assert loaded == stalk, "the same structure, names and leaves"
    ctx = fox_believing()
    assert loaded.tick(ctx) == stalk.tick(ctx)
    wander = Wander().tree
    assert from_xml(to_xml(wander), leaves_of(wander)) == wander


def test_the_checked_in_tree_files_are_what_the_code_exports() -> None:
    files = exports()
    assert set(files) == {"stalk", "wander"}
    for name, text in files.items():
        path = TREES / f"{name}.xml"
        assert path.exists(), f"{path} is missing: run uv run trees"
        assert path.read_text() == text, f"{path} fell behind the code: run uv run trees"


def test_a_file_naming_a_leaf_the_ambition_lacks_says_which() -> None:
    text = to_xml(Selector("t", (Sequence("s", (Condition("is it", lambda c: True), Act("do it", lambda c: None))),)))
    with pytest.raises(UnknownLeaf, match="no condition 'is_it'"):
        from_xml(text, leaves_of(Act("do it", lambda c: None)))
    with pytest.raises(UnknownLeaf, match="no act 'do_it'"):
        from_xml(text, leaves_of(Condition("is it", lambda c: True)))


def test_a_file_that_is_not_a_tree_is_refused_with_a_reason() -> None:
    with pytest.raises(Malformed, match="expected <root>"):
        from_xml("<tree/>", leaves_of(Act("x", lambda c: None)))
    with pytest.raises(Malformed, match="one <BehaviorTree>"):
        from_xml('<root BTCPP_format="4"/>', leaves_of(Act("x", lambda c: None)))
    with pytest.raises(Malformed, match="no node for <Decorator>"):
        from_xml('<root><BehaviorTree ID="t"><Decorator/></BehaviorTree></root>', leaves_of(Act("x", lambda c: None)))
