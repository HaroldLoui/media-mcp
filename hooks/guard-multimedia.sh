#!/bin/bash
set -euo pipefail
# PreToolUse hook: block Read tool on multimedia files
# Reads JSON from stdin, outputs decision JSON to stdout
# No external dependencies (no jq required)

INPUT=$(cat)

# Extract tool_name using grep+sed (no jq dependency)
TOOL_NAME=$(echo "$INPUT" | grep -o '"tool_name"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/"tool_name"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')

# Only intercept Read tool
if [ "$TOOL_NAME" != "Read" ]; then
    exit 0
fi

# Extract file_path from tool_input
FILE_PATH=$(echo "$INPUT" | grep -o '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/')

# Escape backslashes for valid JSON output (Windows paths)
FILE_PATH_ESCAPED=$(echo "$FILE_PATH" | sed 's/\\/\\\\/g')

if [ -z "$FILE_PATH" ]; then
    exit 0
fi

# Get extension (lowercase)
EXT="${FILE_PATH##*.}"
EXT_LOWER=$(echo "$EXT" | tr '[:upper:]' '[:lower:]')

# Blocked extensions list (must match config.json)
BLOCKED_EXTENSIONS=(
    png jpg jpeg gif bmp webp svg ico tiff tif
    mp4 avi mkv mov webm
    mp3 wav flac aac ogg
    pdf
)

for ext in "${BLOCKED_EXTENSIONS[@]}"; do
    if [ ".$EXT_LOWER" = ".$ext" ]; then
        echo "{\"decision\":\"block\",\"reason\":\"Multimedia file detected. Use read_media(file_path=\\\"$FILE_PATH_ESCAPED\\\") from media-mcp tool instead of Read. The Read tool cannot process this file type and will cause an API error.\"}"
        exit 0
    fi
done

exit 0
