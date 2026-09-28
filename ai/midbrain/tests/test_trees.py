"""Trees as files: out to BT.CPP's XML and back, and the checked-in files current."""

from __future__ import annotations

import xml.etree.ElementTree as ET

import pytest

from midbrain.ambitions import HARE, Stalk, Wander
from midbrain.behaviour import Act, Condition, Selector, Sequence
from midbrain.trees import TREES, Malformed, UnknownLeaf, from_xml, identifier, leaves_of, load, outline, to_xml
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


def test_every_leaf_an_ambition_declares_is_in_its_file_and_every_id_in_the_file_is_a_leaf() -> None:
    for ambition in (Stalk(prey=HARE), Wander()):
        tree = ambition.tree  # loads, so every ID in the file is a leaf
        in_file = leaves_of(tree)
        assert set(in_file.conditions) == set(ambition.leaves.conditions), ambition.name
        assert set(in_file.acts) == set(ambition.leaves.acts), ambition.name


def test_a_changed_file_is_read_again_and_a_broken_edit_keeps_the_last_good_tree(tmp_path, capsys) -> None:
    import os
    import time

    leaves = leaves_of(Selector("t", (Condition("is it", lambda c: True), Act("do it", lambda c: None))))
    path = tmp_path / "t.xml"
    path.write_text(to_xml(Selector("t", (Condition("is it", lambda c: True),))))
    first = load("t", leaves, tmp_path)
    assert [child.name for child in first.children] == ["is it"]

    path.write_text(to_xml(Selector("t", (Act("do it", lambda c: None),))))
    os.utime(path, (time.time() + 5, time.time() + 5))
    second = load("t", leaves, tmp_path)
    assert [child.name for child in second.children] == ["do it"], "the edit reached the tree"

    path.write_text("<root><BehaviorTree ID='t'><Fallback name='t'><Action ID='no_such'/></Fallback></BehaviorTree></root>")
    os.utime(path, (time.time() + 10, time.time() + 10))
    third = load("t", leaves, tmp_path)
    assert third == second, "the broken edit is not taken"
    assert "no act 'no_such'" in capsys.readouterr().err
    assert load("t", leaves, tmp_path) == second and capsys.readouterr().err == "", "warned once"

    broken_from_the_start = tmp_path / "u.xml"
    broken_from_the_start.write_text("<root/>")
    with pytest.raises(Malformed):
        load("u", leaves, tmp_path)


def test_an_outline_reads_top_down() -> None:
    lines = outline(Stalk(prey=HARE).tree).splitlines()
    assert lines[0] == "? stalk" and lines[1] == "  → freeze" and lines[2] == "    if looking at us" and lines[3] == "    do stop"
    assert lines[-1] == "  do watch"


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
