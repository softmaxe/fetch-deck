<p align="center">
  <img src="docs/assets/logo.png" width="176" alt="FetchDeck Logo">
</p>

<h1 align="center">FetchDeck</h1>

<p align="center">在 macOS 终端界面中使用 <code>yt-dlp</code> 下载视频、音频和字幕。</p>

<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh-CN.md"><kbd>简体中文</kbd></a>
</p>

<p align="center">
  <img src="docs/assets/demo.gif" width="960" alt="FetchDeck 下载流程演示">
</p>

FetchDeck 用清晰的步骤处理单个视频 URL。粘贴链接、选择输出、确认任务并查看进度，不用自己编写 `yt-dlp` 命令。

## 快速开始

FetchDeck 需要 macOS 和 [Homebrew](https://brew.sh/)。

```sh
brew tap softmaxe/tap
brew install fetchdeck
fetchdeck
```

Homebrew 会同时安装 FetchDeck 所需的 `yt-dlp` 和 `ffmpeg`。

## 支持的功能

| 输出 | 可选项 |
| --- | --- |
| 视频 | MP4，可选择最高可用清晰度、4K、1080p、720p 或 480p |
| 音频 | 提取为 M4A |
| 字幕 | 下载一条手动字幕轨道，保存为 SRT 或 VTT |

FetchDeck 还可以：

- 经确认后读取本机 Chrome、Firefox 或 Brave 配置中的 cookie。
- 显示进度、速度、预计剩余时间、状态和最近的 `yt-dlp` 输出。
- 取消和重试下载，并保留分段文件供 `yt-dlp` 继续使用。
- 在本地保存设置和最近 100 个任务。

每次只能处理一个视频 URL，不支持播放列表和频道 URL。

## 使用流程

1. 粘贴视频 URL。如需 cookie，选择一个浏览器配置。
2. 选择 Video、Audio 或 Subtitles，并设置输出选项。
3. 检查识别到的元数据、格式和保存位置。
4. 开始任务并查看进度。完成后可在 Finder 中打开结果或继续下载。

## 从源码运行

需要 macOS、位于 `PATH` 中的稳定版 Rust 和 `cargo`，以及 `yt-dlp`、`ffmpeg`。

```sh
brew install yt-dlp ffmpeg
git clone https://github.com/softmaxe/fetch-deck.git
cd fetch-deck
cargo run
```

构建优化版本：

```sh
cargo build --release
./target/release/fetchdeck
```

如果缺少依赖，FetchDeck 会在界面顶部提示。你也可以在 Settings 中指定 `yt-dlp` 和 `ffmpeg` 的路径。

## 操作方式

| 按键 | 操作 |
| --- | --- |
| `j` / `k`、上 / 下方向键 | 在字段间移动或滚动内容 |
| 左 / 右方向键 | 更改当前选项 |
| Page Up / Page Down | 将 Review 或 Progress 日志滚动十行 |
| `Enter` | 继续、开始任务或编辑设置 |
| `Esc` | 返回、停止读取元数据或关闭面板 |
| `c` | 取消当前下载 |
| `n` / `r` / `o` | 新建任务、重试或在 Finder 中打开结果 |
| `F1` / `F2` / `F3` | 打开 Help、History 或 Settings |
| `x` | 在 History 面板中清除记录，不会删除下载文件 |
| `e` / `s` | 在 Settings 面板中编辑或保存设置 |
| `q` | 退出，下载进行中时需要确认 |

也可以使用鼠标。点击字段和选项进行操作，使用滚轮浏览 Review 和 Progress。

## Cookie、隐私和本地数据

FetchDeck 会先征得同意，再读取浏览器 cookie。第一次成功读取时，本机的 `yt-dlp` 会将所选配置中的 cookie 导出到私有临时文件。FetchDeck 在当前会话中复用该文件，并在退出时删除。

日志和错误信息会隐藏 cookie 文件路径及浏览器认证详情。设置和历史记录不会保存所选浏览器、配置、cookie 文件或生成的命令。FetchDeck 不发送遥测数据。

Settings 保存输出目录以及可选的 `yt-dlp`、`ffmpeg` 路径。History 最多保存 100 个任务的 URL、标题、结果、输出路径和时间戳。清空 History 不会删除下载文件。macOS 会将两个文件保存在 `com.softmaxe.fetchdeck` 的标准应用目录中。

部分浏览器会在运行时锁定 cookie 数据库。如果 FetchDeck 无法读取，请关闭所选浏览器后重试。

## 当前限制

- 不支持播放列表、频道、站内搜索和自定义 `yt-dlp` 参数
- 不支持 MP3 转换
- 不支持内嵌字幕和自动生成字幕
- 不支持暂停、后台下载和跨应用会话自动续传
- 不支持 Safari、Edge cookie 和 `cookies.txt` 导入
- FetchDeck 不内置或自动更新 `yt-dlp` 和 `ffmpeg`

## 重新生成演示动图

演示使用固定的离线元数据和通用 `/tmp/fetchdeck-demo-*` 路径，不会访问浏览器配置、真实视频 URL 或当前用户的下载目录。

```sh
brew install vhs
cargo build --release
vhs docs/demo/demo.tape
```

Tape 使用 JetBrains Mono 16 和 Catppuccin Mocha。VHS 使用不透明背景，因此不包含 Ghostty 的模糊和透明效果。

## 许可证

[AGPL-3.0](LICENSE)
