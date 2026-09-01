<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh-CN.md"><kbd>简体中文</kbd></a>
</p>

<h1 align="center">FetchDeck</h1>

FetchDeck is a terminal interface for common `yt-dlp` downloads on macOS. It handles one URL at a time and walks through a fixed workflow:

`Source` → `Probe` → `Options` → `Review` → `Progress` → `Done`

`Probe` reads the title, formats, and manual subtitle tracks before the app presents the available choices. The interface stays focused on the usual download cases instead of exposing every `yt-dlp` argument.

## What it supports

- One video URL per download. Playlist and channel URLs are rejected.
- Video downloads remuxed to MP4.
- Audio extraction as M4A.
- One manual subtitle track converted to SRT or VTT. Embedded and automatically generated subtitles are not included.
- Quality choices derived from the probed formats. `Best available`, `1080p`, `720p`, and `480p` appear when applicable. `4K` appears only when the source has a format at or above 2160p.
- Cookies from local Chrome, Firefox, and Brave profiles.
- One active download, with progress, speed, ETA, status, and a bounded raw log.
- Cancellation and retry. Partial files remain available for `yt-dlp` to continue on retry.
- Local settings and up to 100 recent history entries.

## Requirements

- macOS
- Stable Rust with `cargo` on `PATH`
- `yt-dlp`
- `ffmpeg`

Install the runtime dependencies with Homebrew:

```sh
brew install yt-dlp ffmpeg
```

## Run from source

```sh
cargo run
```

The header shows the detected paths for `yt-dlp` and `ffmpeg`. You can override both paths in Settings.

For a release build:

```sh
cargo build --release
./target/release/fetchdeck
```

## Controls

| Key | Action |
| --- | --- |
| `j` / `k`, Up / Down | Move through fields or scroll Review and the Progress log |
| Left / Right | Change the selected option |
| Page Up / Page Down | Scroll Review or the Progress log by ten lines |
| `Enter` | Continue, start a download, begin a new download, or edit a setting |
| `Esc` | Go back, stop reading metadata, or close a panel |
| `c` | Cancel the active download |
| `n` | Begin a new download from Done |
| `r` | Retry a failed or cancelled download from Done |
| `o` | Open the output in Finder from Done |
| `F1` / `F2` / `F3` | Open Help, History, or Settings |
| `x` | Clear History while its panel is open. Downloaded files are untouched |
| `e` / `s` | Edit or save Settings while its panel is open |
| `q` | Quit. An active download requires a second confirmation |

Mouse movement highlights clickable fields and actions. Click a text field to focus it, click a choice to advance it, and use the wheel to scroll Review or Progress.

## Browser cookies

The first browser-cookie selection asks for confirmation. On the first successful Probe for a browser and profile, the app asks the local `yt-dlp` executable to read that profile and export the cookies to a private temporary Netscape cookie jar. On macOS, the jar is created with owner-only permissions. Later probes, downloads, and retries for the same browser and profile reuse it during that app session.

The temporary directory and jar are removed when the app exits. The jar path and browser authentication details are scrubbed from displayed logs and errors. Config and history do not store the browser, profile, cookie jar, or generated command. The app sends no telemetry.

Some browsers lock their cookie database while running. If Probe reports a cookie database access error, close the selected browser and retry.

## Settings and history

Settings stores the output directory and optional paths to `yt-dlp` and `ffmpeg`. History stores the URL, title, result, output path, and timestamp for the latest 100 completed, failed, or cancelled downloads. Press `x` in History to clear these entries without deleting downloaded files.

Both files live in the standard macOS application directories selected for `com.softmaxe.fetchdeck`.

## Limitations

- No playlist or channel downloads
- No site search or advanced `yt-dlp` arguments
- No MP3 conversion
- No embedded or automatically generated subtitles
- No pause, background downloads, or automatic cross-session resume
- No Safari or Edge cookies
- No `cookies.txt` import
- No bundled or automatically updated `yt-dlp` or `ffmpeg`

## License

[MIT](LICENSE)
