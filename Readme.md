# poker-cfr

Implementation of counterfactual regret minimization (CFR) for Texas hold'em poker

## Algorithm

- CFR+
- Supports multi-thread
- Precomputed heads-up equity
- Written in Rust (fast and safe)

## Files

- `cfr.rs`

The main logic of counterfactual regret minimization is described.

- `main_kuhn.rs` (`$ cargo run --release`)

Solve Nash equilibrium of Kuhn poker (mainly for testing).

- `main_push_fold.rs` (`$ cargo run --release --bin push_fold`)

Solve Nash equilibrium of heads-up push/fold hold'em, i.e., the heads-up poker only allowed to push (all-in) or fold.

- `main_preflop.rs` (`$ cargo run --release --bin preflop`)

Solve Nash equilibrium of pre-flop only heads-up hold'em, i.e., every player checks after flop opens. Currently, the bet size is limited to 2.5x, 3x, 3.5x, 4x, and all-in.

- `main_viewer.rs` (`$ cargo run --release --bin viewer`)

Open a CUI interactive viewer for pre-flop strategies computed by `main_preflop.rs`. It reads data in `output` directory.
