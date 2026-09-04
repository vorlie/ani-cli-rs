#!/usr/bin/env sh
set -eu

repository="vorlie/ani-cli-rs"
api_url="https://api.github.com/repos/$repository/releases/latest"
host_os=$(uname -s)
host_arch=$(uname -m)

case "${TERMUX_VERSION:-}:${PREFIX:-}" in
    ?*:*|*:*com.termux*)
        echo "Official Termux binaries are not provided. Install rust, then build ani-cli-rs from source." >&2
        exit 2
        ;;
esac

case "$host_os:$host_arch" in
    Linux:x86_64) target=x86_64-unknown-linux-musl ;;
    Linux:aarch64|Linux:arm64)
        echo "Official Linux ARM64 binaries are not provided. Install Rust and build ani-cli-rs from source." >&2
        exit 2
        ;;
    Darwin:*) echo "Official macOS binaries are not provided. Install Rust and build ani-cli-rs from source." >&2; exit 2 ;;
    *) echo "ani-cli-rs does not publish a build for $host_os/$host_arch." >&2; exit 2 ;;
esac

for command_name in curl tar; do
    command -v "$command_name" >/dev/null 2>&1 || {
        echo "Required command '$command_name' was not found." >&2
        exit 2
    }
done

release_json=$(curl -fsSL -H "Accept: application/vnd.github+json" -H "User-Agent: ani-cli-rs-installer" "$api_url")
version=$(printf '%s\n' "$release_json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)
version_number=${version#v}
asset_name="ani-cli-rs-$version_number-$target.tar.gz"
asset_url="https://github.com/$repository/releases/download/$version/$asset_name"
checksum_url="$asset_url.sha256"

if [ -z "$version" ]; then
    echo "Could not determine the latest ani-cli-rs release version." >&2
    exit 1
fi

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT HUP INT TERM
archive_path="$temporary_directory/$asset_name"
checksum_path="$archive_path.sha256"

echo "Downloading ani-cli-rs $version for $target..."
curl -fL --retry 3 -o "$archive_path" "$asset_url"
curl -fL --retry 3 -o "$checksum_path" "$checksum_url"

expected_hash=$(sed -n '1s/[[:space:]].*//p' "$checksum_path")
if command -v sha256sum >/dev/null 2>&1; then
    actual_hash=$(sha256sum "$archive_path" | sed 's/[[:space:]].*//')
else
    actual_hash=$(shasum -a 256 "$archive_path" | sed 's/[[:space:]].*//')
fi
if [ "$expected_hash" != "$actual_hash" ]; then
    echo "Checksum verification failed for $asset_name." >&2
    exit 1
fi

tar -xzf "$archive_path" -C "$temporary_directory"
binary_path=$(find "$temporary_directory" -type f -name ani-cli-rs | head -n 1)
if [ -z "$binary_path" ]; then
    echo "The release archive did not contain ani-cli-rs." >&2
    exit 1
fi

install_directory=${ANI_CLI_RS_INSTALL_DIR:-"$HOME/.local/bin"}
mkdir -p "$install_directory"
install -m 755 "$binary_path" "$install_directory/ani-cli-rs"

case ":$PATH:" in
    *":$install_directory:"*) ;;
    *)
        # Detect target profile based on active shell or standard rc files
        if [ -n "${ANI_CLI_RS_PROFILE:-}" ]; then
            profile="$ANI_CLI_RS_PROFILE"
        elif [ -n "${ZSH_VERSION:-}" ] || [ -f "$HOME/.zshrc" ]; then
            profile="$HOME/.zshrc"
        elif [ -n "${BASH_VERSION:-}" ] || [ -f "$HOME/.bashrc" ]; then
            profile="$HOME/.bashrc"
        elif [ -f "$HOME/.config/fish/config.fish" ]; then
            profile="$HOME/.config/fish/config.fish"
        else
            profile="$HOME/.profile"
        fi

        {
            echo ""
            echo "# ani-cli-rs installer"
            if echo "$profile" | grep -q "config.fish"; then
                echo "fish_add_path $install_directory"
            else
                echo "export PATH=\"$install_directory:\$PATH\""
            fi
        } >> "$profile"

        echo "Added $install_directory to PATH in $profile."
        echo "Run 'source $profile' or restart your shell to apply changes."
        ;;
esac

echo "Installed ani-cli-rs $version to $install_directory/ani-cli-rs"
