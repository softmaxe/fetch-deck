<p align="center">
  <img src="docs/assets/logo.png" width="176" alt="FetchDeck logo">
</p>

<h1 align="center">FetchDeck</h1>

<p align="center">A focused macOS terminal interface for everyday <code>yt-dlp</code> downloads.</p>

<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh-CN.md"><kbd>简体中文</kbd></a>
</p>

<p align="center">
  <img src="docs/assets/demo.gif" width="960" alt="FetchDeck workflow demo">
</p>

FetchDeck guides one video URL through a short workflow. Paste the URL, choose the output, review the generated job, and watch the transfer without memorizing `yt-dlp` flags.

## Features

- Download video and remux it to MP4.
- Extract audio as M4A.
- Download one manual subtitle track as SRT or VTT.
- Pick from qualities detected in the source: Best available, 4K, 1080p, 720p, or 480p.
- Read cookies from local Chrome, Firefox, or Brave profiles after confirmation.
- Follow progress, speed, ETA, status, and a bounded raw log.
- Cancel a transfer and retry it with partial files left in place for `yt-dlp`.
- Keep local settings and the latest 100 history entries.

FetchDeck accepts one video URL at a time. It rejects playlist and channel URLs.

## Install with Homebrew

```sh
brew tap softmaxe/tap
brew install fetchdeck
```

The formula installs `yt-dlp` and `ffmpeg` as dependencies.

## Run from source

Source builds require:

- macOS
- Stable Rust with `cargo` on `PATH`
- `yt-dlp`
- `ffmpeg`

Install the runtime tools with Homebrew:

```sh
brew install yt-dlp ffmpeg
```

```sh
git clone https://github.com/softmaxe/fetch-deck.git
cd fetch-deck
cargo run
```

For an optimized build:

```sh
cargo build --release
./target/release/fetchdeck
```

The header reports whether FetchDeck found `yt-dlp` and `ffmpeg`. You can override either executable in Settings.

## Workflow

| Step | What happens |
| --- | --- |
| Source | Paste a video URL and choose whether to use browser cookies. FetchDeck probes the title, formats, and manual subtitle tracks. |
| Options | Choose Video, Audio, or Subtitles, then set quality, subtitle format, and output directory. |
| Review | Check the source, selected format, metadata, and destination before running anything. |
| Progress | Follow the transfer gauge, speed, ETA, status, and raw `yt-dlp` output. |
| Done | Open the result in Finder, start another download, or retry a failed or cancelled job. |

## Controls

| Key | Action |
| --- | --- |
| `j` / `k`, Up / Down | Move between fields or scroll Review and the Progress log |
| Left / Right | Change the selected option |
| Page Up / Page Down | Scroll Review or the Progress log by ten lines |
| `Enter` | Continue, start a download, begin a new download, or edit a setting |
| `Esc` | Go back, stop the metadata probe, or close a panel |
| `c` | Cancel the active download |
| `n` | Start a new download from Done |
| `r` | Retry a failed or cancelled download from Done |
| `o` | Open the output in Finder from Done |
| `F1` / `F2` / `F3` | Open Help, History, or Settings |
| `x` | Clear History while its panel is open; downloaded files stay untouched |
| `e` / `s` | Edit or save Settings while its panel is open |
| `q` | Quit; an active download requires a second confirmation |

Mouse input also works. Click a field to focus it, click a choice to advance it, and use the wheel to scroll Review or Progress.

## Browser cookies and privacy

FetchDeck asks before reading browser cookies. On the first successful probe for a browser profile, the local `yt-dlp` executable exports its cookies to a private, temporary Netscape cookie jar. Later probes, downloads, and retries for the same profile reuse that jar until FetchDeck exits.

The temporary directory and cookie jar are deleted on exit. Displayed logs and errors scrub the jar path and browser authentication details. Settings and history do not store the browser, profile, cookie jar, or generated command. FetchDeck sends no telemetry.

Some browsers lock their cookie database while running. If the probe cannot access it, close the selected browser and retry.

## Local data

Settings contain the output directory and optional paths to `yt-dlp` and `ffmpeg`. History contains the URL, title, result, output path, and timestamp for up to 100 completed, failed, or cancelled jobs. Clearing History does not delete downloaded files.

macOS stores both files in the standard application directories for `com.softmaxe.fetchdeck`.

## Rebuild the demo

The animation uses fixed, offline metadata and generic `/tmp/fetchdeck-demo-*` paths. It does not access a browser profile, a real video URL, or the current user's download directory.

The tape matches the reference Ghostty profile: JetBrains Mono 16 and Catppuccin Mocha. VHS renders an opaque background, so Ghostty's blur and transparency are intentionally omitted.

```sh
brew install vhs
cargo build --release
vhs docs/demo/demo.tape
```

## Limitations

- No playlists or channels
- No site search or advanced `yt-dlp` arguments
- No MP3 conversion
- No embedded or automatically generated subtitles
- No pause, background downloads, or automatic cross-session resume
- No Safari or Edge cookies
- No `cookies.txt` import
- No bundled or automatically updated `yt-dlp` or `ffmpeg`

## License

[MIT](LICENSE)
