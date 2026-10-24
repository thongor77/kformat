#!/usr/bin/env bash
# Installs kformat for the current user only (no root): binary, launcher, device action.
#   scripts/install-user.sh              build (release) and install
#   scripts/install-user.sh --uninstall  remove everything this script installed
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="${XDG_BIN_HOME:-$HOME/.local/bin}"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"

binary="$bin_dir/kformat"
launcher="$data_dir/applications/kformat.desktop"
action="$data_dir/solid/actions/kformat-format.desktop"

if [[ "${1:-}" == "--uninstall" ]]; then
    rm -fv "$binary" "$launcher" "$action"
    exit 0
fi

cd "$root"
cargo build --release -p kformat-app

# Plasma only watches Solid action directories that exist when it starts.
first_action_dir=false
[[ -d "$(dirname "$action")" ]] || first_action_dir=true

install -Dm755 target/release/kformat-app "$binary"
install -Dm644 packaging/kformat.desktop "$launcher"
install -Dm644 packaging/solid/kformat-format.desktop "$action"

# Make sure Exec= resolves even when ~/.local/bin is not on the desktop's PATH.
sed -i "s|^Exec=kformat|Exec=$binary|" "$launcher" "$action"

echo "Installed:"
printf '  %s\n' "$binary" "$launcher" "$action"
if $first_action_dir; then
    echo "Note: $(dirname "$action") is new. Restart the shell so the device notifier"
    echo "      picks it up:  systemctl --user restart plasma-plasmashell.service"
    echo "      (or log out and back in)."
fi
case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) echo "Note: $bin_dir is not in PATH; the launcher uses the full path." ;;
esac
