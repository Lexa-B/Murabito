#!/usr/bin/env bash
# Open the project in the editor.
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
require_display
exec "$UE_EDITOR" "$PROJECT" "$@"
