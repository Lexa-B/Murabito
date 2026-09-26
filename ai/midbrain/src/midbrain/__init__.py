"""The midbrain: the slow mind, outside the game, on its own clock.

It reads every body's snapshot from the game's bridge and tells each body what to want.
``client`` is the wire; ``murabito_pb2`` the contract, generated from
``crates/ai/bridge/proto/murabito.proto``; ``board`` a live view of what a mind is told.
"""
