<p align="center">
  <img src="../assets/logo.png" width="176" alt="FetchDeck 标志">
</p>

<h1 align="center">录制 FetchDeck 演示</h1>

<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh-CN.md"><kbd>简体中文</kbd></a>
</p>

<p align="center">重新生成仓库中 <code>docs/assets/demo.gif</code> 的离线终端演示。</p>

<p align="center">
  <img src="../assets/demo.gif" width="960" alt="FetchDeck 离线演示">
</p>

## 录制演示

安装 [VHS](https://github.com/charmbracelet/vhs)，构建 FetchDeck，然后在仓库根目录运行 tape：

```sh
brew install vhs
cargo build --release
vhs docs/demo/demo.tape
```

`demo.tape` 会录制一个 120 列、30 行的终端，并写入 `docs/assets/demo.gif`。`run-demo.sh` 会在 `/tmp` 下为 FetchDeck 创建隔离的主目录，将模拟的 `yt-dlp` 和 `ffmpeg` 命令放在 `PATH` 最前面，并在每次运行前后清理演示文件。它不会读取用户的 FetchDeck 设置、浏览器数据或下载目录。

`demo.tape` 使用 JetBrains Mono 16 和 Catppuccin Mocha 配色。VHS 会渲染不透明背景，无法还原 Ghostty 的模糊、透明效果或自定义 cursor shader。

## 使用其他二进制文件

如需使用其他已构建的二进制文件：

```sh
FETCHDECK_BIN=/path/to/fetchdeck docs/demo/run-demo.sh
```
