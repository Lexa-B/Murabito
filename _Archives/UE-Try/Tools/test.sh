#!/usr/bin/env bash
# Build a private copy of the project, then run its automation tests headless. Usage: test.sh [TestPathPrefix]
# Exits 0 only if at least one test ran, none failed, none were skipped or left
# not-run, and the automation queue's own completion count matches the passed count.
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
FILTER="${1:-Murabito}"

# Copy the project's source, config and content into the test mirror and build it there without hot
# reload, so the tests run exactly the code on disk and never touch an open editor's modules.
mkdir -p "$TEST_MIRROR"
rsync -a --delete --exclude Content/Developers --exclude Content/Collections \
  "$ROOT_DIR/Source" "$ROOT_DIR/Config" "$ROOT_DIR/Content" "$TEST_MIRROR/"
cp "$PROJECT" "$TEST_MIRROR/Murabito.uproject"
MIRROR_PROJECT="$TEST_MIRROR/Murabito.uproject"
echo "test mirror: $TEST_MIRROR"
"$BUILD_SH" MurabitoEditor Linux Development -Project="$MIRROR_PROJECT" -WaitMutex -NoHotReloadFromIDE

LOG_DIR="$ROOT_DIR/Saved/TestLogs"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/test-$(date +%Y%m%d-%H%M%S).log"

# timeout wraps the editor process this script itself starts, so it's safe to kill on expiry.
set +e
timeout 1200 "$UE_EDITOR_CMD" "$MIRROR_PROJECT" \
  -ExecCmds="Automation RunTests $FILTER" \
  -TestExit="Automation Test Queue Empty" \
  -unattended -nullrhi -nosplash -nosound -stdout -FullStdOutLogOutput \
  > "$LOG" 2>&1
CODE=$?
set -e

if [[ "$CODE" -eq 124 ]]; then
  echo "log: $LOG"
  echo "TESTS FAILED: editor timed out after 1200s"
  exit 1
fi

rg --no-line-number "Test Completed\. Result=" "$LOG" || true
PASSED=$(rg -c "Test Completed\. Result=\{Success\}" "$LOG" || true)
FAILED=$(rg -c "Test Completed\. Result=\{Fail" "$LOG" || true)
PASSED=${PASSED:-0}
FAILED=${FAILED:-0}

# Skipped/NotRun tests would otherwise pass this gate silently (FAILED stays 0). UE's automation
# state enum (Engine/Source/Runtime/AutomationTest/Public/AutomationState.h) renders through
# UEnum display-name formatting, which spaces camel-case names, so "NotRun" logs as "Not Run"
# while "Skipped" (one word) is unchanged.
SKIPPED_OR_NOTRUN=$(rg -c "Test Completed\. Result=\{(Skipped|Not Run)\}" "$LOG" || true)
SKIPPED_OR_NOTRUN=${SKIPPED_OR_NOTRUN:-0}

# The completion count is the automation queue's own tally, from a line such as:
#   LogAutomationCommandLine: Display: ...Automation Test Queue Empty 18 tests performed.
# Anchor on "tests performed", not "Automation Test Queue Empty" alone: the early
# "LogInit: Command Line:" echo of our own -TestExit argument also contains that phrase, but
# only the real summary line contains "tests performed". A crash partway through the run (or a
# hang caught by the timeout above) leaves this line absent, which we treat as failure.
COMPLETED=$(rg -o "[0-9]+ tests performed" "$LOG" | rg -o "^[0-9]+" | head -1 || true)

echo "passed: $PASSED  failed: $FAILED  completed: ${COMPLETED:-none}  skipped/not-run: $SKIPPED_OR_NOTRUN  editor exit code: $CODE"
echo "log: $LOG"
# Note: the editor's own exit code is not a pass/fail signal here. -TestExit force-exits via
# FUnixPlatformMisc::RequestExit(true, ...), which always _exit(1)s on that path regardless of
# test outcome (see Engine/Source/Runtime/Core/Private/Unix/UnixPlatformMisc.cpp). Pass/fail is
# decided from the automation log instead.
if [[ -z "$COMPLETED" || "$FAILED" -ne 0 || "$PASSED" -eq 0 || "$COMPLETED" -ne "$PASSED" || "$SKIPPED_OR_NOTRUN" -ne 0 ]]; then
  rg --no-line-number "Error: |Expected " "$LOG" | head -40 || true
  echo "TESTS FAILED"
  exit 1
fi
echo "TESTS PASSED"
