#!/usr/bin/env bash
# Builds and starts Ortelia with its log in a new folder; once the window is closed, prints
# the warnings and errors of that log and the exit code.
#
# Usage: tools/launch.sh [--release] [--check] [-- ORTELIA ARGUMENTS...]
#   ORTELIA ARGUMENTS  default: examples/annuaire (paths are relative to the repository root)
#   --release  optimized build (for performance checks)
#   --check    automatic check instead of an interactive run: the smoke test
#              (crates/ortelia/tests/smoke.rs) starts Ortelia on examples/annuaire and
#              checks its log
#
# Linux only: on Windows, run it in WSL2 (`wsl -- bash -l tools/launch.sh`); WSLg shows the
# window on the Windows desktop.
set -u

cd "$(dirname "$0")/.." || exit 2

# A repository on a Windows drive (/mnt/c/...) builds much faster with the artifacts in the
# Linux file system.
if [[ -z "${CARGO_TARGET_DIR:-}" && "$PWD" == /mnt/* ]]; then
    export CARGO_TARGET_DIR="$HOME/target/ortelia"
fi

profile=debug
build_flags=()
check=0
app_args=()
while (($# > 0)); do
    case "$1" in
        --release)
            profile=release
            build_flags=(--release)
            ;;
        --check) check=1 ;;
        --)
            shift
            app_args=("$@")
            break
            ;;
        -h | --help)
            sed -n '/^# Usage:/,/^#$/p' "$0"
            exit 0
            ;;
        *)
            echo "launch.sh: unknown option: $1 (Ortelia arguments go after --)" >&2
            exit 2
            ;;
    esac
    shift
done

if ((check)); then
    if ((${#app_args[@]} > 0)); then
        echo "launch.sh: the smoke test always opens examples/annuaire, without other arguments" >&2
        exit 2
    fi
    exec cargo test --locked -p ortelia "${build_flags[@]}" --test smoke -- --ignored
fi

cargo build --locked -p ortelia "${build_flags[@]}" || exit 1

# Without arguments, the example of the repository.
if ((${#app_args[@]} == 0)); then
    app_args=(examples/annuaire)
fi

# A new log folder per run: the log then only holds the lines of this run.
log_dir="$(mktemp -d -t ortelia-launch.XXXXXX)"
echo "log folder: $log_dir"
ORTELIA_LOG_DIR="$log_dir" "${CARGO_TARGET_DIR:-target}/$profile/ortelia" "${app_args[@]}"
status=$?

echo
echo "=== warnings and errors of the log"
# Line format: <timestamp> <LEVEL> [<spans>:] <target>: <message>
grep -hE '^[^ ]+ +(WARN|ERROR) ' "$log_dir"/* 2>/dev/null || echo "(none)"
echo
echo "Ortelia exited with code $status"
exit "$status"
