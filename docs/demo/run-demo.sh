#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
demo_home=/tmp/fetchdeck-demo-home
demo_bin="$demo_home/bin"
fetchdeck_bin=${FETCHDECK_BIN:-"$repo_root/target/release/fetchdeck"}

cleanup() {
  rm -rf "$demo_home" /tmp/fetchdeck-demo-output
}

trap cleanup EXIT HUP INT TERM

if [ ! -x "$fetchdeck_bin" ]; then
  printf 'FetchDeck binary not found: %s\n' "$fetchdeck_bin" >&2
  printf 'Build it first with: cargo build --release\n' >&2
  exit 1
fi

cleanup
mkdir -p "$demo_bin" /tmp/fetchdeck-demo-output
cp "$repo_root/docs/demo/fake-yt-dlp" "$demo_bin/yt-dlp"
cp "$repo_root/docs/demo/fake-ffmpeg" "$demo_bin/ffmpeg"
chmod 700 "$demo_bin/yt-dlp" "$demo_bin/ffmpeg"

export HOME="$demo_home"
export XDG_CONFIG_HOME="$demo_home/config"
export XDG_DATA_HOME="$demo_home/data"
export PATH="$demo_bin:/usr/bin:/bin"

"$fetchdeck_bin"
