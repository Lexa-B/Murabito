# midbrain

The slow mind, outside the game, in Python. It talks to the game's bridge on
`127.0.0.1:15703` in the contract `crates/ai/bridge/proto/murabito.proto`.

```
uv run board          # watch every body's snapshot, live, while the game runs
uv run order 3 walkto 0 0 # tell body #3 what to want: stop | face DIR | face-thing N | walkto Q R (or jogto, sprintto, sneakto)
uv run order 1 recoil W   # or walk, jog, sprint, sneak, sidestep, backstep, lunge DIR; bite
uv run mind           # the mind: believes, decides, sends; what each body believes and decided, live
uv run mind --visualize   # the same, with one body's believed world in a window; --body N, Tab cycles, Esc quits
uv run pytest         # the tests
./regen.sh            # after the .proto changes: regenerate murabito_pb2.py
```

`src/midbrain/client.py` is the wire: frames, a request for snapshots, an order.
`order.py` says one order by hand; `board.py` shows what every body is told.
`beliefs.py` is the believed world: a `BelievedWorld` per body, a `Belief` per thing it has
seen, frozen where it was last seen. `ambitions.py` is what a body can want (`Idle`, `Wander`, `Stalk`),
each bidding a utility and the winner running a tree from `behaviour.py`; `hexes.py` is the
geometry and `paths.py` the planner, A* across the plane round what is believed. `mind.py`
runs the round: observe, revise, choose, want, send what differs from what the body is doing,
a path as an amendment of the one in hand; `visualize.py` draws one body's world, decision
and route, flat, north up, in the game's geometry. Run the game beside it: the game is the
truth.
`murabito_pb2.py` is generated and checked in; a test fails if it falls behind the
contract. Python 3.13, dependencies with `uv add`.
