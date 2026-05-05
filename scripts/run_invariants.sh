#!/usr/bin/env bash
# scripts/run_invariants.sh — runs the Phase-1 physics-invariant suite locally.
#
# Builds the binary in release mode, then dispatches every test in turn.
# Each test writes doe_results/physics_validation/<name>/result.json and
# compares against the committed baseline at
# tests/physics_invariants/baselines/<name>.json.
#
# Exit code: 0 if every implemented test passes, 1 if any fails.
# (Stub tests print "not yet implemented" and count as pass for now.)

set -uo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

TESTS=(
  charge_balance
  zero_emf_symmetric
  driven_symmetric
  nve_energy_drift
  mb_velocity_distribution
  quadtree_force_error
  no_spurious_plating
)

echo "== building physics_invariants =="
cargo build --release --bin physics_invariants

failed=0
for t in "${TESTS[@]}"; do
  echo
  echo "== running $t =="
  if ! cargo run --release --quiet --bin physics_invariants -- --test "$t"; then
    echo "FAIL: $t"
    failed=$((failed + 1))
  fi
done

echo
if (( failed == 0 )); then
  echo "All implemented invariants passed."
  exit 0
else
  echo "$failed invariant(s) failed."
  exit 1
fi
