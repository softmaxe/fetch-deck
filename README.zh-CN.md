<p align="center">
  <img src="docs/assets/logo.png" width="176" alt="FetchDeck Logo">
</p>

<h1 align="center">FetchDeck</h1>

<p align="center">面向日常 <code>yt-dlp</code> 下载场景的 macOS 终端界面。</p>

<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh-CN.md"><kbd>简体中文</kbd></a>
</p>

<p align="center">
  <img src="docs/assets/demo.gif" width="960" alt="FetchDeck 完整流程演示">
</p>

FetchDeck 用一个简短流程处理单个视频 URL。粘贴链接，选择输出格式，确认任务，然后在终端里查看下载进度，不用背 `yt-dlp` 参数。

## 功能

- 下载视频并重封装为 MP4。
- 提取音频为 M4A。
- 下载一条手动字幕轨道，并转换为 SRT 或 VTT。
- 根据源视频提供 Best available、4K、1080p、720p 或 480p 清晰度。
- 经确认后读取本机 Chrome、Firefox 或 Brave 配置中的 cookie。
- 显示进度、速度、预计剩余时间、状态和长度受限的原始日志。
- 取消并重试下载，保留分段文件供 `yt-dlp` 续传。
- 在本地保存设置和最近 100 条历史记录。

FetchDeck 每次接受一个视频 URL，播放列表和频道 URL 会被拒绝。

## 使用 Homebrew 安装

```sh
brew tap softmaxe/tap
brew install fetchdeck
```

Formula 会自动安装 `yt-dlp` 和 `ffmpeg` 依赖。

## 从源码运行

从源码构建需要：

- macOS
- 稳定版 Rust，且 `cargo` 位于 `PATH`
- `yt-dlp`
- `ffmpeg`

使用 Homebrew 安装运行时工具：

```sh
brew install yt-dlp ffmpeg
```

```sh
git clone https://github.com/softmaxe/FetchDeck.git
cd FetchDeck
cargo run
```

构建优化版本：

```sh
cargo build --release
./target/release/fetchdeck
```

界面顶部会显示 FetchDeck 是否找到 `yt-dlp` 和 `ffmpeg`。你可以在 Settings 中覆盖任一可执行文件的路径。

## 下载流程

| 阶段 | 操作 |
| --- | --- |
| Source | 粘贴视频 URL，并选择是否使用浏览器 cookie。FetchDeck 会读取标题、格式和手动字幕轨道。 |
| Options | 选择 Video、Audio 或 Subtitles，再设置清晰度、字幕格式和输出目录。 |
| Review | 执行前检查来源、所选格式、元数据和保存位置。 |
| Progress | 查看进度条、速度、预计剩余时间、状态和原始 `yt-dlp` 输出。 |
| Done | 在 Finder 中打开结果、新建下载，或重试失败及已取消的任务。 |

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
| `q` | 退出；下载进行中时需要再次确认 |

鼠标也可以操作。点击字段可将其聚焦，点击选项可切换到下一项，滚轮可滚动 Review 或 Progress。

## 浏览器 cookie 和隐私

FetchDeck 会先征得同意，再读取浏览器 cookie。某个浏览器配置第一次成功完成探测时，本机 `yt-dlp` 会把 cookie 导出到私有的 Netscape 格式临时 cookie jar。同一应用会话中，后续探测、下载和重试会复用该文件。

FetchDeck 退出时会删除临时目录和 cookie jar。界面日志和错误信息会隐藏 jar 路径及浏览器认证详情。设置和历史记录不会保存浏览器、配置名称、cookie jar 或生成的命令。FetchDeck 不发送遥测数据。

浏览器运行时可能锁定 cookie 数据库。如果探测阶段无法访问它，请关闭所选浏览器后重试。

## 本地数据

Settings 保存输出目录以及可选的 `yt-dlp`、`ffmpeg` 路径。History 保存最近 100 个已完成、失败或取消的任务，包括 URL、标题、结果、输出路径和时间戳。清空 History 不会删除下载文件。

macOS 会将两个文件保存在 `com.softmaxe.fetchdeck` 的标准应用目录中。

## 重新生成演示动图

动图使用固定的离线元数据和通用 `/tmp/fetchdeck-demo-*` 路径，不会访问浏览器配置、真实视频 URL 或当前用户的下载目录。

Tape 使用参考 Ghostty 配置：JetBrains Mono 16 和 Catppuccin Mocha。VHS 使用不透明背景，因此不会复现 Ghostty 的模糊和透明效果。

```sh
brew install vhs
cargo build --release
vhs docs/demo/demo.tape
```

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
