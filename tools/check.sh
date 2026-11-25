#!/usr/bin/env bash
# Runs locally the checks of the CI job `checks` (.github/workflows/ci.yml), plus Clippy for
# the Windows target, which the CI only gets on its Windows runner.
#
# Usage: tools/check.sh [--no-smoke] [--no-windows]
#   --no-smoke    skip the smoke test of the real window
#   --no-windows  skip Clippy for x86_64-pc-windows-msvc
#
# Every step runs, even after a failure; a summary follows, and the exit code is 1 if a step
# failed. Linux only: on Windows, run it in WSL2 (`wsl -- bash -l tools/check.sh`).
set -u

cd "$(dirname "$0")/.." || exit 2

# A repository on a Windows drive (/mnt/c/...) builds much faster with the artifacts in the
# Linux file system.
if [[ -z "${CARGO_TARGET_DIR:-}" && "$PWD" == /mnt/* ]]; then
    export CARGO_TARGET_DIR="$HOME/target/ortelia"
fi

smoke=1
windows=1
for arg in "$@"; do
    case "$arg" in
        --no-smoke) smoke=0 ;;
        --no-windows) windows=0 ;;
        -h | --help)
            sed -n '/^# Usage:/,/^#$/p' "$0"
            exit 0
            ;;
        *)
            echo "check.sh: unknown option: $arg" >&2
            exit 2
            ;;
    esac
done

results=()
failed=0

# step NAME COMMAND...: runs the command and records its result and duration.
step() {
    local name="$1"
    shift
    echo
    echo "=== $name"
    local start=$SECONDS
    if "$@"; then
        results+=("ok       $name ($((SECONDS - start)) s)")
    else
        fail "$name" "$((SECONDS - start)) s"
    fi
}

# fail NAME DETAIL: records a failed step.
fail() {
    results+=("FAILED   $1 ($2)")
    failed=1
}

# skip NAME REASON: records a step that did not run.
skip() {
    results+=("skipped  $1 ($2)")
}

step "format" cargo fmt --all --check
step "clippy" cargo clippy --workspace --all-targets --locked -- -D warnings

# Code reserved to Linux by #[cfg] is dead code under Windows: only this pass sees its lints.
# It only type-checks (no link), so the MSVC tools are not needed.
if ((windows)); then
    if rustup target list --installed | grep -qx x86_64-pc-windows-msvc; then
        step "clippy (Windows target)" cargo clippy --workspace --all-targets --locked \
            --target x86_64-pc-windows-msvc -- -D warnings
    else
        fail "clippy (Windows target)" "missing target: rustup target add x86_64-pc-windows-msvc"
    fi
else
    skip "clippy (Windows target)" "--no-windows"
fi

step "tests" cargo test --workspace --locked
step "layer rule" cargo xtask check-layers
step "JSON Schemas" cargo xtask gen-schemas --check

if ((smoke)); then
    if [[ -n "${DISPLAY:-}${WAYLAND_DISPLAY:-}" ]]; then
        step "smoke test" cargo test -p ortelia --locked --test smoke -- --ignored
    elif command -v xvfb-run >/dev/null; then
        step "smoke test (Xvfb)" xvfb-run -a cargo test -p ortelia --locked --test smoke -- --ignored
    else
        skip "smoke test" "no display and no xvfb-run"
    fi
else
    skip "smoke test" "--no-smoke"
fi

echo
echo "=== summary"
printf '%s\n' "${results[@]}"
exit "$failed"
