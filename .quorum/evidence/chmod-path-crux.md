# Crux lane — PMAT-204 — how the field validates a declared file mode

Analyzed configuration management file mode handling from documentation memory and generated an architectural design document for forjar.

## What forjar took from it

The panel's recommendation — validate the mode into a typed value before emitting shell, in the Nix posture, rather than re-deriving a linter's verdict afterwards — is the direction of `world_writable_modes`, which refuses a world-writable mode at the gate under forjar's own code. Its premise that shell linters parse an AST and cannot confuse a path digit for a mode is refuted by this ticket's own measurement, and is marked as such. Every third-party figure in the lane is [X] (documentation memory, no network).
