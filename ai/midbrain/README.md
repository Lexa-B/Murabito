# midbrain

The slow mind, outside the game, in Python. It talks to the game's bridge on
`127.0.0.1:15703` in the contract `crates/ai/bridge/proto/murabito.proto`.

```
uv run board          # watch every body's snapshot, live, while the game runs
uv run pytest         # the tests
./regen.sh            # after the .proto changes: regenerate murabito_pb2.py
```

`src/midbrain/client.py` is the wire: frames, a request for snapshots, an order.
`murabito_pb2.py` is generated and checked in; a test fails if it falls behind the
contract. Python 3.13, dependencies with `uv add`.
