<p align="center">
  <img src="../assets/logo.png" width="176" alt="FetchDeck logo">
</p>

<h1 align="center">Record the FetchDeck demo</h1>

<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh-CN.md"><kbd>简体中文</kbd></a>
</p>

<p align="center">Rebuild the offline terminal demo committed at <code>docs/assets/demo.gif</code>.</p>

<p align="center">
  <img src="../assets/demo.gif" width="960" alt="FetchDeck offline demo">
</p>

## Record the demo

Install [VHS](https://github.com/charmbracelet/vhs), build FetchDeck, then run the tape from the repository root:

```sh
brew install vhs
cargo build --release
vhs docs/demo/demo.tape
```

The tape records a 120-column by 30-row terminal and writes `docs/assets/demo.gif`. `run-demo.sh` gives FetchDeck an isolated home directory under `/tmp`, puts fake `yt-dlp` and `ffmpeg` commands first on `PATH`, and clears its demo files before and after each run. It does not read the user's FetchDeck settings, browser data, or download directory.

The tape requests JetBrains Mono 16 and uses the Catppuccin Mocha palette. VHS renders an opaque background and does not reproduce Ghostty blur, transparency, or custom cursor shaders.

## Use another binary

To use another prebuilt binary:

```sh
FETCHDECK_BIN=/path/to/fetchdeck docs/demo/run-demo.sh
```
