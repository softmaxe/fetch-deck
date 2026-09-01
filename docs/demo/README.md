# Recording the demo

Build FetchDeck, then run the tape from the repository root:

```sh
cargo build --release
vhs docs/demo/demo.tape
```

The tape records a 120-column by 30-row terminal and writes
`docs/assets/demo.gif`. `run-demo.sh` gives FetchDeck an isolated home directory
under `/tmp`, puts the fake `yt-dlp` first on `PATH`, and clears its demo files
before each run. It does not read the user's FetchDeck config or browser data.

The committed style mirrors the reference Ghostty profile with JetBrains Mono
16 and the Catppuccin Mocha palette. VHS does not reproduce Ghostty's blur,
transparency, or custom cursor shader.

To use another prebuilt binary:

```sh
FETCHDECK_BIN=/path/to/fetchdeck docs/demo/run-demo.sh
```
