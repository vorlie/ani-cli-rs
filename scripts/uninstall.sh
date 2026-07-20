#!/usr/bin/env sh
set -eu

install_directory=${ANI_CLI_RS_INSTALL_DIR:-"$HOME/.local/bin"}
binary_path="$install_directory/ani-cli-rs"
profile=${ANI_CLI_RS_PROFILE:-"$HOME/.profile"}

if [ -f "$binary_path" ]; then
    rm -f "$binary_path"
    echo "Removed $binary_path"
fi

if [ -f "$profile" ]; then
    temporary_profile=$(mktemp)
    awk '
        previous == "# ani-cli-rs installer" && /^export PATH=/ { previous=""; next }
        { if (previous != "") print previous; previous=$0 }
        END { if (previous != "") print previous }
    ' "$profile" > "$temporary_profile"
    mv "$temporary_profile" "$profile"
fi

echo "ani-cli-rs has been uninstalled."

