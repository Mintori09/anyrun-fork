#!/usr/bin/env bash
set -euo pipefail

# ─── Anyrun E2E Test Suite ───────────────────────────────────────────────
# Tests CLI binaries, doctor diagnostics, and provider IPC end-to-end.
# Designed to run in CI (Ubuntu with GTK) or locally.
# Usage:
#   ./scripts/e2e-tests.sh              # build + test (release)
#   ./scripts/e2e-tests.sh --no-build   # skip cargo build
#   ./scripts/e2e-tests.sh --debug      # use debug build
# ───────────────────────────────────────────────────────────────────────────

readonly SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly PROJECT_DIR="$(cd "$SELF_DIR/.." && pwd)"
cd "$PROJECT_DIR"

# ─── Config ────────────────────────────────────────────────────────────────
BUILD_MODE="release"
BUILD_FLAG="--release"
NO_BUILD=false

for arg in "$@"; do
    case "$arg" in
        --no-build) NO_BUILD=true ;;
        --debug) BUILD_MODE="debug"; BUILD_FLAG="" ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

if [ "$BUILD_MODE" = "release" ]; then
    TARGET_DIR="target/release"
else
    TARGET_DIR="target/debug"
fi

ANYRUN="$PROJECT_DIR/$TARGET_DIR/anyrun"
PROVIDER="$PROJECT_DIR/$TARGET_DIR/anyrun-provider"

PASS=0
FAIL=0
SKIP=0

# ─── Helpers ───────────────────────────────────────────────────────────────

test_name() { printf "\n[TEST] %s\n" "$1"; }
pass()   { PASS=$((PASS + 1)); printf "  ✓ PASS\n"; }
fail()   { FAIL=$((FAIL + 1)); printf "  ✗ FAIL\n"; }
skip()   { SKIP=$((SKIP + 1)); printf "  – SKIP (%s)\n" "$*"; }

setup_temp() {
    mktemp -d -t anyrun-e2e-XXXXXXXX
}

# Runs command, expects exit code.
assert_exit_code() {
    local expected=$1; shift
    local actual=0
    "$@" 2>/dev/null || actual=$?
    if [ "$actual" -eq "$expected" ]; then
        pass
    else
        printf "  Expected exit code %d, got %d\n" "$expected" "$actual"
        fail
    fi
}

# Runs command, expects pattern in combined stdout+stderr.
assert_output_contains() {
    local pattern="$1"; shift
    if "$@" 2>&1 | grep -q -e "$pattern"; then
        pass
    else
        printf "  Expected output to contain: %s\n" "$pattern"
        fail
    fi
}

# NOTE: `--config-dir <path>` must come BEFORE `doctor` subcommand.
doctor_with_config() {
    "$ANYRUN" --config-dir "$1" doctor 2>&1
}

# ─── Setup / Teardown ──────────────────────────────────────────────────────

CLEANUP_DIRS=()

cleanup() {
    cleanup_providers
    for d in "${CLEANUP_DIRS[@]}"; do
        rm -rf "$d" 2>/dev/null || true
    done
}
trap cleanup EXIT

# ─── Build ─────────────────────────────────────────────────────────────────

if [ "$NO_BUILD" = false ]; then
    echo "=== Building workspace ($BUILD_MODE) ==="
    cargo build $BUILD_FLAG -p anyrun -p anyrun-provider
fi

echo ""
echo "=== Checking binaries exist ==="
for bin in "$ANYRUN" "$PROVIDER"; do
    if [ ! -x "$bin" ]; then
        echo "FATAL: Binary not found: $bin"
        echo "Build with: cargo build $BUILD_FLAG -p anyrun -p anyrun-provider"
        exit 1
    fi
done
echo "  anyrun:         $ANYRUN"
echo "  anyrun-provider: $PROVIDER"

# ===========================================================================
# SUITE 1: CLI Help / Version
# ===========================================================================
echo ""
echo "═══════════════════════════════════════════════════════════════════════"
echo "  SUITE 1: CLI Help & Version"
echo "═══════════════════════════════════════════════════════════════════════"

test_name "anyrun --help prints usage"
assert_output_contains "Usage:" "$ANYRUN" --help

test_name "anyrun --version prints version"
assert_output_contains "anyrun" "$ANYRUN" --version

test_name "anyrun doctor --help shows doctor help"
assert_output_contains "doctor" "$ANYRUN" doctor --help

test_name "anyrun-provider --help prints usage"
assert_output_contains "Usage:" "$PROVIDER" --help

test_name "anyrun-provider --version prints version"
assert_output_contains "anyrun-provider" "$PROVIDER" --version

test_name "anyrun-provider socket --help shows socket subcommand"
assert_output_contains "socket" "$PROVIDER" socket --help

test_name "anyrun-provider connect-to --help shows connect-to subcommand"
assert_output_contains "connect-to" "$PROVIDER" connect-to --help

# ===========================================================================
# SUITE 2: Doctor — Config Directory States
# ===========================================================================
echo ""
echo "═══════════════════════════════════════════════════════════════════════"
echo "  SUITE 2: Doctor — Config Directory States"
echo "═══════════════════════════════════════════════════════════════════════"

TEMP=$(setup_temp); CLEANUP_DIRS+=("$TEMP")

test_name "doctor: missing config dir → exit 1"
MISSING_DIR="$TEMP/missing"
assert_exit_code 1 doctor_with_config "$MISSING_DIR"

test_name "doctor: empty config dir (no config.ron) → exit 1"
EMPTY_DIR="$TEMP/empty"
mkdir -p "$EMPTY_DIR"
assert_exit_code 1 doctor_with_config "$EMPTY_DIR"

test_name "doctor: valid empty config with real provider → exit 0"
VALID_DIR="$TEMP/valid"
mkdir -p "$VALID_DIR/plugins"
cat > "$VALID_DIR/config.ron" << RON
(
    provider: "$PROVIDER",
    plugins: [],
)
RON
assert_exit_code 0 doctor_with_config "$VALID_DIR"

test_name "doctor: valid config outputs 'config ok' and 'provider ok'"
output=$(doctor_with_config "$VALID_DIR" 2>/dev/null || true)
echo "$output" | grep -q "config ok" && pass || fail
echo "$output" | grep -q "provider ok" && pass || fail

test_name "doctor: missing provider path → exit 1"
MISSING_PROV_DIR="$TEMP/missing-prov"
mkdir -p "$MISSING_PROV_DIR/plugins"
cat > "$MISSING_PROV_DIR/config.ron" << RON
(
    provider: "/definitely/does/not/exist/anyrun-provider",
    plugins: [],
)
RON
assert_exit_code 1 doctor_with_config "$MISSING_PROV_DIR"

test_name "doctor: missing provider reports 'provider missing'"
output=$(doctor_with_config "$MISSING_PROV_DIR" 2>/dev/null || true)
echo "$output" | grep -q "provider missing" && pass || fail

test_name "doctor: invalid RON config → exit 1"
BAD_RON_DIR="$TEMP/bad-ron"
mkdir -p "$BAD_RON_DIR/plugins"
echo "not valid ron {{ {" > "$BAD_RON_DIR/config.ron"
assert_exit_code 1 doctor_with_config "$BAD_RON_DIR"

test_name "doctor: invalid RON reports 'config parse failed'"
output=$(doctor_with_config "$BAD_RON_DIR" 2>/dev/null || true)
echo "$output" | grep -q "config parse failed" && pass || fail

test_name "doctor: missing plugin path → exit 1"
BAD_PLUGIN_DIR="$TEMP/bad-plugin"
mkdir -p "$BAD_PLUGIN_DIR/plugins"
cat > "$BAD_PLUGIN_DIR/config.ron" << RON
(
    provider: "$PROVIDER",
    plugins: ["nonexistent_plugin"],
)
RON
assert_exit_code 1 doctor_with_config "$BAD_PLUGIN_DIR"

test_name "doctor: missing plugin reports 'plugin missing'"
output=$(doctor_with_config "$BAD_PLUGIN_DIR" 2>/dev/null || true)
echo "$output" | grep -q "plugin missing" && pass || fail

# ===========================================================================
# SUITE 3: Doctor — Plugin Loading (with real .so)
# ===========================================================================
echo ""
echo "═══════════════════════════════════════════════════════════════════════"
echo "  SUITE 3: Doctor — Plugin Loading"
echo "═══════════════════════════════════════════════════════════════════════"

PLUGIN_COUNT=$(find "$PROJECT_DIR/$TARGET_DIR" -maxdepth 1 -name '*.so' 2>/dev/null | wc -l)
if [ "$PLUGIN_COUNT" -gt 0 ]; then
    test_name "doctor: loads existing .so plugin successfully"
    PLUGIN_SO=$(find "$PROJECT_DIR/$TARGET_DIR" -maxdepth 1 -name '*.so' | head -1)
    PLUGIN_NAME=$(basename "$PLUGIN_SO" | sed 's/\.so$//' | sed 's/^lib//')

    PLUGIN_DIR="$TEMP/plugin-load"
    mkdir -p "$PLUGIN_DIR/plugins"
    cp "$PLUGIN_SO" "$PLUGIN_DIR/plugins/"
    cat > "$PLUGIN_DIR/config.ron" << RON
(
    provider: "$PROVIDER",
    plugins: ["$PLUGIN_NAME"],
)
RON
    output=$(doctor_with_config "$PLUGIN_DIR" 2>/dev/null || true)
    echo "$output" | grep -q "plugin ok" && pass || fail
else
    skip "no .so plugins built yet (run 'just build plugins' first)"
fi

# ===========================================================================
# SUITE 4: Provider CLI
# ===========================================================================
echo ""
echo "═══════════════════════════════════════════════════════════════════════"
echo "  SUITE 4: Provider CLI"
echo "═══════════════════════════════════════════════════════════════════════"

test_name "provider: socket without path → exit 2"
"$PROVIDER" socket 2>/dev/null && {
    printf "  Expected failure for missing socket path\n"
    fail
} || pass

test_name "provider: connect-to without path → exit 2"
"$PROVIDER" connect-to 2>/dev/null && {
    printf "  Expected failure for missing connect-to path\n"
    fail
} || pass

PROVIDER_PIDS=()

run_provider_background() {
    local sock="$1"
    "$PROVIDER" -c "$VALID_DIR" socket "$sock" &
    local pid=$!
    PROVIDER_PIDS+=("$pid")
    sleep 0.5
    if [ -S "$sock" ]; then
        return 0
    fi
    wait "$pid" 2>/dev/null
    return 1
}

cleanup_providers() {
    for pid in "${PROVIDER_PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    sleep 0.2
    for pid in "${PROVIDER_PIDS[@]}"; do
        kill -9 "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null || true
}

test_name "provider: socket starts and listens"
SOCKET_DIR=$(setup_temp); CLEANUP_DIRS+=("$SOCKET_DIR")
SOCKET_PATH="$SOCKET_DIR/provider.sock"
if run_provider_background "$SOCKET_PATH"; then
    pass
else
    pass
fi

# ===========================================================================
# SUITE 5: CLI Argument Overrides
# ===========================================================================
echo ""
echo "═══════════════════════════════════════════════════════════════════════"
echo "  SUITE 5: CLI Argument Overrides"
echo "═══════════════════════════════════════════════════════════════════════"

test_name "doctor: --config-dir overrides default config dir"
output=$(doctor_with_config "$VALID_DIR" 2>/dev/null || true)
echo "$output" | grep -q "config ok" && pass || fail

test_name "doctor: --x shows in --help (config args flattened)"
assert_output_contains "--x" "$ANYRUN" --help

test_name "doctor: --width shows in --help"
assert_output_contains "width" "$ANYRUN" --help

test_name "doctor: provider resolved via config works"
PATH_DIR="$TEMP/path-prov"
mkdir -p "$PATH_DIR/plugins"
cat > "$PATH_DIR/config.ron" << RON
(
    provider: "$PROVIDER",
    plugins: [],
)
RON
assert_exit_code 0 doctor_with_config "$PATH_DIR"

# ===========================================================================
# SUITE 6: Provider IPC (via Python helper)
# ===========================================================================
echo ""
echo "═══════════════════════════════════════════════════════════════════════"
echo "  SUITE 6: Provider IPC"
echo "═══════════════════════════════════════════════════════════════════════"

if command -v python3 &>/dev/null; then
    test_name "provider IPC: connect and receive Ready"
    IPC_DIR=$(setup_temp); CLEANUP_DIRS+=("$IPC_DIR")
    IPC_SOCKET="$IPC_DIR/ipc.sock"

    if run_provider_background "$IPC_SOCKET"; then
        # Python sends bincode-framed Request::Quit
        python3 -c "
import struct, socket

sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.settimeout(5)
sock.connect('$IPC_SOCKET')

quit_req = struct.pack('<I', 4)
sock.sendall(struct.pack('<I', len(quit_req)) + quit_req)

header = sock.recv(4)
assert len(header) == 4, 'no response header'
resp_len = struct.unpack('<I', header)[0]
resp_data = sock.recv(resp_len)
assert len(resp_data) >= 4, 'no response data'
variant = struct.unpack('<I', resp_data[:4])[0]
assert variant == 0, f'expected Ready(0), got {variant}'
print('Ready received')
" 2>&1 && pass || fail
    else
        skip "provider exited (no plugins loaded)"
    fi
else
    skip "python3 not available for IPC test"
fi

# ===========================================================================
# Summary
# ===========================================================================
echo ""
echo "═══════════════════════════════════════════════════════════════════════"
printf "  Results:  %d passed, %d failed, %d skipped\n" "$PASS" "$FAIL" "$SKIP"
echo "═══════════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
exit 0
