<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh-CN.md"><kbd>简体中文</kbd></a>
</p>

<h1 align="center">FetchDeck</h1>

FetchDeck 是一个用于 macOS 的 `yt-dlp` 终端界面。它每次处理一个 URL，并按照固定流程完成下载：

`Source` → `Probe` → `Options` → `Review` → `Progress` → `Done`

`Probe` 会先读取标题、格式和手动字幕轨道，再由界面列出可选项。这个界面只覆盖常见下载场景，不会展示 `yt-dlp` 的全部参数。

## 支持的功能

- 每次下载一个视频 URL。播放列表和频道 URL 会被拒绝。
- 下载视频并重封装为 MP4。
- 提取音频为 M4A。
- 下载一条手动字幕轨道并转换为 SRT 或 VTT。不包括内嵌字幕和自动生成字幕。
- 根据探测到的格式提供清晰度选项。适用时会显示 `Best available`、`1080p`、`720p` 和 `480p`。只有源视频存在 2160p 或更高格式时才显示 `4K`。
- 读取本机 Chrome、Firefox 和 Brave 配置中的 cookie。
- 同时运行一个下载，显示进度、速度、预计剩余时间、状态和长度受限的原始日志。
- 取消和重试下载。重试时，`yt-dlp` 可以继续使用未完成的分段文件。
- 在本地保存设置和最近 100 条历史记录。

## 环境要求

- macOS
- 稳定版 Rust，且 `cargo` 位于 `PATH`
- `yt-dlp`
- `ffmpeg`

使用 Homebrew 安装运行时依赖：

```sh
brew install yt-dlp ffmpeg
```

## 从源码运行

```sh
cargo run
```

界面顶部会显示检测到的 `yt-dlp` 和 `ffmpeg` 路径。你可以在 Settings 中覆盖这两个路径。

构建并运行 release 版本：

```sh
cargo build --release
./target/release/fetchdeck
```

## 操作方式

| 按键 | 操作 |
| --- | --- |
| `j` / `k`、上 / 下方向键 | 在字段间移动，或滚动 Review 和 Progress 日志 |
| 左 / 右方向键 | 更改当前选项 |
| Page Up / Page Down | 将 Review 或 Progress 日志滚动十行 |
| `Enter` | 继续、开始下载、新建下载或编辑设置 |
| `Esc` | 返回、停止读取元数据或关闭面板 |
| `c` | 取消当前下载 |
| `n` | 在 Done 页面新建下载 |
| `r` | 在 Done 页面重试失败或已取消的下载 |
| `o` | 在 Done 页面用 Finder 打开输出文件 |
| `F1` / `F2` / `F3` | 打开 Help、History 或 Settings |
| `x` | 在 History 面板中清除历史记录，不会删除已下载文件 |
| `e` / `s` | 在 Settings 面板中编辑或保存设置 |
| `q` | 退出。下载进行中时需要再次确认 |

移动鼠标会高亮可点击的字段和操作。点击文本字段可将其聚焦，点击选项可切换到下一项，滚轮可滚动 Review 或 Progress。

## 浏览器 cookie

第一次选择浏览器 cookie 时，应用会要求确认。某个浏览器和配置第一次成功完成 Probe 时，应用会让本机 `yt-dlp` 读取该配置，并将 cookie 导出到私有的 Netscape 格式临时 cookie jar。在 macOS 上，该文件仅允许当前用户访问。同一应用会话中，之后对相同浏览器和配置进行 Probe、下载或重试时都会复用这个文件。

应用退出时会删除临时目录和 cookie jar。界面日志和错误信息会隐藏 jar 路径及浏览器认证详情。配置和历史记录不会保存浏览器、配置名称、cookie jar 或生成的命令。应用不会发送遥测数据。

浏览器运行时可能锁定 cookie 数据库。如果 Probe 报告无法访问 cookie 数据库，请关闭所选浏览器后重试。

## 设置和历史记录

Settings 保存输出目录以及可选的 `yt-dlp`、`ffmpeg` 路径。History 保存最近 100 个已完成、失败或取消的下载，包括 URL、标题、结果、输出路径和时间戳。在 History 中按 `x` 会清空这些记录，不会删除已下载文件。

两个文件都位于 macOS 为 `com.softmaxe.fetchdeck` 分配的标准应用目录中。

## 当前限制

- 不支持播放列表或频道下载
- 不支持站内搜索或高级 `yt-dlp` 参数
- 不支持 MP3 转换
- 不支持内嵌字幕或自动生成字幕
- 不支持暂停、后台下载或跨会话自动续传
- 不支持 Safari 或 Edge cookie
- 不支持导入 `cookies.txt`
- 不内置或自动更新 `yt-dlp` 和 `ffmpeg`

## 许可证

[MIT](LICENSE)
