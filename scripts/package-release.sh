#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
host_os=$(uname -s)
host_arch=$(uname -m)

if [ "$#" -gt 0 ]; then
    target=$1
else
    case "$host_os:$host_arch" in
        Linux:x86_64) target=x86_64-unknown-linux-musl ;;
        Linux:aarch64|Linux:arm64) target=aarch64-unknown-linux-musl ;;
        Darwin:*) echo "Official macOS release packages are not provided; build locally with cargo build --release." >&2; exit 2 ;;
        *) echo "Unsupported host $host_os/$host_arch; pass a Rust target triple explicitly." >&2; exit 2 ;;
    esac
fi

version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_root/Cargo.toml" | head -n 1)
package_name="ani-cli-rs-$version-$target"
package_directory="$project_root/dist/$package_name"
archive_path="$project_root/dist/$package_name.tar.gz"
binary_path="$project_root/target/$target/release/ani-cli-rs"

if [ -f "$project_root/LICENSE" ]; then
    license_path="$project_root/LICENSE"
elif [ -f "$project_root/../../LICENSE" ]; then
    license_path="$project_root/../../LICENSE"
else
    echo "Could not find LICENSE in the project or repository root." >&2
    exit 2
fi

cargo build --locked --release --target "$target" --manifest-path "$project_root/Cargo.toml"
mkdir -p "$package_directory"
cp "$binary_path" "$package_directory/ani-cli-rs"
cp "$project_root/README.md" "$package_directory/README.md"
cp "$license_path" "$package_directory/LICENSE"
chmod 755 "$package_directory/ani-cli-rs"

tar -C "$project_root/dist" -czf "$archive_path" "$package_name"
(
    cd "$project_root/dist"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$package_name.tar.gz" > "$package_name.tar.gz.sha256"
    else
        shasum -a 256 "$package_name.tar.gz" > "$package_name.tar.gz.sha256"
    fi
)
echo "Created $archive_path and $archive_path.sha256"
