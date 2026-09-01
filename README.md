# yt-dlp-tui

`yt-dlp-tui` is a macOS terminal interface for common `yt-dlp` downloads. It keeps the queue, format choices, progress, and errors visible without exposing the full command-line option set.

## First release scope

- Download one video URL per job. Playlist and channel downloads are rejected.
- Probe metadata and formats before adding a job.
- Download video as MP4, audio as M4A, or one subtitle track as SRT/VTT.
- Offer `Best available`, `4K`, `1080p`, `720p`, and `480p` when the source provides them. The `4K` choice only appears for sources with a 2160p format.
- Read cookies through local Chrome, Firefox, or Brave profiles.
- Run one download at a time and keep later jobs queued.
- Show progress, speed, ETA, processing stage, and bounded raw logs.
- Cancel or retry a job. Partial files remain available to `yt-dlp` for a retry.
- Save non-sensitive settings and recent history locally.

## Requirements

- macOS
- Rust stable with `cargo` on `PATH`
- `yt-dlp`
- `ffmpeg`

Homebrew can install the runtime dependencies:

```sh
brew install yt-dlp ffmpeg
```

## Run from source

```sh
cargo run
```

The first screen reports the detected `yt-dlp` and `ffmpeg` paths. Settings can override either path.

## Controls

| Key | Action |
| --- | --- |
| `a` | Add a job |
| `j` / `k`, arrow keys | Move through jobs or fields |
| `Tab` / `Shift-Tab` | Move between form fields |
| Left / Right | Change the selected option |
| `Enter` | Probe, review, confirm, or edit |
| `c` | Cancel the selected queued or active job |
| `r` | Retry a failed or cancelled job |
| `o` | Open the selected output in Finder |
| `1` / `2` / `3` | Open Queue, History, or Settings |
| `?` | Open Help |
| `q` | Quit; active jobs require a second confirmation |

The first browser-cookie selection asks for confirmation. Closing the selected browser before retrying can resolve cookie database access errors.

## Privacy

The application starts the local `yt-dlp` executable with the selected browser profile. It does not export cookies, store cookie values, or send telemetry. History excludes browser, profile, cookie, and command details.

## Not in the first release

- Playlist and channel downloads
- Site search
- Advanced `yt-dlp` arguments
- MP3 conversion
- Embedded or automatically generated subtitles
- Pause, background downloads, or automatic cross-session resume
- Safari or Edge cookies
- `cookies.txt` import
- Bundled or automatically updated `yt-dlp` and `ffmpeg`

## License

MIT
