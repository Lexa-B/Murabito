#!/usr/bin/env bash
# Build the editor target (Development, Linux). Extra args go to UnrealBuildTool.
# With an Unreal Editor open, this is a hot-reload build that the open editor can pick up.
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
note_running_editor
"$BUILD_SH" MurabitoEditor Linux Development -Project="$PROJECT" -WaitMutex "$@"
