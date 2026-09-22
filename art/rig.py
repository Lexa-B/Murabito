"""Skeletons for models that bend: named bones in an armature, and each vertex of
a mesh bound to one or more of them, exported with the model as a glTF skin.

Drape points are the simplest kind: a bone standing upright at a spot on the
ground, which the game raises to the terrain height beneath its resting spot,
so whatever rides on it settles onto uneven ground. A DrapeGrid lays them out
under a wide, low shape and blends each vertex between the four around it.

    r = rig.Rig("KuzuRig")
    r.bone("root", (0, 0, 0), (0, 0, 1))
    names = r.chain("runner_0", points, parent="root")  # a bone per segment
    r.weigh(vertex_indices, names[2])                    # while building
    obj = builder.finish("Kuzu")
    r.bind(obj, default="root")                          # every other vertex rides on root

Vertex indices are the order the Builder made the vertices in, which is the
order they end up in the mesh: note len(builder.bm.verts) before and after
building a part to know its vertices. loft.run exports a mesh's rig with it.
"""

import math

import bpy


class Rig:
    def __init__(self, name):
        self.obj = bpy.data.objects.new(name, bpy.data.armatures.new(name))
        bpy.context.scene.collection.objects.link(self.obj)
        self._bones = []  # (name, head, tail, parent, connected)
        self._weights = {}  # vertex index -> [(bone, weight)]

    def bone(self, name, head, tail, parent=None, connected=False):
        self._bones.append((name, tuple(head), tuple(tail), parent, connected))
        return name

    def chain(self, prefix, points, parent=None):
        """A bone from each point to the next, each joined to the one before;
        the first hangs from parent. Returns the bones' names, <prefix>_0 on."""
        names = []
        for k, (head, tail) in enumerate(zip(points, points[1:])):
            names.append(self.bone(f"{prefix}_{k}", head, tail, names[-1] if names else parent, bool(names)))
        return names

    def drape_point(self, name, x, y, parent="root"):
        """A bone standing up at (x, y) on the ground: the game raises it to the
        terrain height there."""
        return self.bone(name, (x, y, 0), (x, y, 1), parent)

    def weigh_blend(self, indices, pairs):
        """Bind these vertices to several bones at once: pairs of (bone, weight)."""
        for bone, weight in pairs:
            self.weigh(indices, bone, weight)

    def weigh(self, indices, bone, weight=1.0):
        """Bind these vertices to bone; a vertex given several bones is shared
        between them in proportion to the weights."""
        for index in indices:
            self._weights.setdefault(index, []).append((bone, weight))

    def bind(self, mesh_obj, default):
        """Build the bones, give mesh_obj a vertex group per bone with the weights
        recorded, bind any vertex not weighed to default, and parent it to the
        rig with an armature modifier."""
        bpy.context.view_layer.objects.active = self.obj
        bpy.ops.object.mode_set(mode="EDIT")
        edit_bones = self.obj.data.edit_bones
        for name, head, tail, parent, connected in self._bones:
            bone = edit_bones.new(name)
            bone.head, bone.tail = head, tail
            if parent:
                bone.parent = edit_bones[parent]
                bone.use_connect = connected
        bpy.ops.object.mode_set(mode="OBJECT")

        groups = {name: mesh_obj.vertex_groups.new(name=name) for name, *_ in self._bones}
        for index in range(len(mesh_obj.data.vertices)):
            pairs = self._weights.get(index, [(default, 1.0)])
            total = sum(w for _, w in pairs)
            for bone, w in pairs:
                groups[bone].add([index], w / total, "ADD")
        mesh_obj.parent = self.obj
        mesh_obj.modifiers.new("rig", "ARMATURE").object = self.obj


class DrapeGrid:
    """Drape points on a grid about spacing apart, covering x0..x1 and y0..y1,
    named <name>_<column>_<row>. weights(x, y) blends a spot between the four
    points around it, so what rides on the grid sags and bulges smoothly
    between them."""

    def __init__(self, skeleton, name, x0, x1, y0, y1, spacing, parent="root"):
        self.x0, self.y0 = x0, y0
        self.columns = max(2, math.ceil((x1 - x0) / spacing) + 1)
        self.rows = max(2, math.ceil((y1 - y0) / spacing) + 1)
        self.dx = (x1 - x0) / (self.columns - 1)
        self.dy = (y1 - y0) / (self.rows - 1)
        self.names = [[skeleton.drape_point(f"{name}_{i}_{j}", x0 + i * self.dx, y0 + j * self.dy, parent)
                       for j in range(self.rows)] for i in range(self.columns)]

    def weights(self, x, y):
        u = min(max((x - self.x0) / self.dx, 0), self.columns - 1)
        v = min(max((y - self.y0) / self.dy, 0), self.rows - 1)
        i, j = min(int(u), self.columns - 2), min(int(v), self.rows - 2)
        u, v = u - i, v - j
        pairs = [(self.names[i][j], (1 - u) * (1 - v)), (self.names[i + 1][j], u * (1 - v)),
                 (self.names[i][j + 1], (1 - u) * v), (self.names[i + 1][j + 1], u * v)]
        return [(bone, w) for bone, w in pairs if w > 1e-6]


def drape_skin(skeleton, verts, pieces, spacing, name="blanket"):
    """Drape a wide, low skin: a DrapeGrid under its vertices' footprint, each
    vertex blended between the four points around it, and each piece riding on
    it rigidly, pieces being (first vertex, end vertex, the spot it grows from)
    for leaves and the like, so they move without warping. Returns the grid."""
    xs, ys = [v.co.x for v in verts], [v.co.y for v in verts]
    grid = DrapeGrid(skeleton, name, min(xs), max(xs), min(ys), max(ys), spacing)
    for v in verts:
        skeleton.weigh_blend([v.index], grid.weights(v.co.x, v.co.y))
    for first, end, spot in pieces:
        skeleton.weigh_blend(range(first, end), grid.weights(spot.x, spot.y))
    return grid
