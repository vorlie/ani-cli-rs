#!/usr/bin/env sh
set -eu

project_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
host_os=$(uname -s)
host_arch=$(uname -m)

target=""
features=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --features)
            if [ "$#" -lt 2 ]; then
                echo "Missing value for --features." >&2
                exit 2
            fi
            features="$2"
            shift 2
            ;;
        --feature)
            if [ "$#" -lt 2 ]; then
                echo "Missing value for --feature." >&2
                exit 2
            fi
            features="$2"
            shift 2
            ;;
        *)
            if [ -z "$target" ]; then
                target=$1
            else
                echo "Unknown argument: $1" >&2
                exit 2
            fi
            shift
            ;;
    esac
done

if [ -z "$target" ]; then
    case "$host_os:$host_arch" in
        Linux:x86_64) target=x86_64-unknown-linux-musl ;;
        Linux:aarch64|Linux:arm64) target=aarch64-unknown-linux-musl ;;
        Darwin:*) echo "Official macOS release packages are not provided; build locally with cargo build --release." >&2; exit 2 ;;
        *) echo "Unsupported host $host_os/$host_arch; pass a Rust target triple explicitly." >&2; exit 2 ;;
    esac
fi

has_gui=false
if [ -n "$features" ] && printf '%s' "$features" | grep -Eq '(^|,|[[:space:]])gui([[:space:]]|,|$)'; then
    has_gui=true
fi

if [ "$has_gui" = true ] && printf '%s' "$target" | grep -Eq -- '-musl$'; then
    echo "The GUI build requires a GNU Linux target (for example x86_64-unknown-linux-gnu). The selected target '$target' is not supported by winit." >&2
    exit 2
fi

if [ "$has_gui" = true ] && [ "$host_os" = "Linux" ] && printf '%s' "$target" | grep -Eq -- '^x86_64-unknown-linux-musl$|^aarch64-unknown-linux-musl$'; then
    case "$host_arch" in
        x86_64) target=x86_64-unknown-linux-gnu ;;
        aarch64|arm64) target=aarch64-unknown-linux-gnu ;;
    esac
fi

version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project_root/Cargo.toml" | head -n 1)
package_name="ani-cli-rs-$version-$target"
package_directory="$project_root/dist/$package_name"
archive_path="$project_root/dist/$package_name.tar.gz"

binary_names="ani-cli-rs"

if [ -f "$project_root/LICENSE" ]; then
    license_path="$project_root/LICENSE"
elif [ -f "$project_root/../../LICENSE" ]; then
    license_path="$project_root/../../LICENSE"
else
    echo "Could not find LICENSE in the project or repository root." >&2
    exit 2
fi

set -- build --locked --release --target "$target" --manifest-path "$project_root/Cargo.toml"
if [ -n "$features" ]; then
    set -- "$@" --features "$features"
fi

cargo "$@"
if [ "$has_gui" = true ]; then
    binary_names="$binary_names ani-cli-rs-gui"
fi

mkdir -p "$package_directory"
for binary_name in $binary_names; do
    binary_path="$project_root/target/$target/release/$binary_name"
    if [ ! -f "$binary_path" ]; then
        echo "Expected release binary not found: $binary_path" >&2
        exit 2
    fi
    cp "$binary_path" "$package_directory/$binary_name"
    chmod 755 "$package_directory/$binary_name"
done
cp "$project_root/README.md" "$package_directory/README.md"
cp "$license_path" "$package_directory/LICENSE"

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
