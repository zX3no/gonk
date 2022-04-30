<h1 align="center" style="font-size: 55px">Gonk</h1>
<h3 align="center">A simple terminal music player.</h3>

<div align="center" style="display:inline">
      <img src="media/gonk.gif">
</div>

## ✨ Features

- Vim-style key bindings
- Easy to use
- Fuzzy search
- Mouse support
- Cross-platform
- Plays FLAC, MP3, OGG, M4A and WAV

## 📦 Installation

> MacOS has not been testing.

> I recommend using a font with ligatures for the best experience.

### crates.io

```
cargo install gonk
```

### From source

Install the dependencies.

```
git clone https://github.com/zX3no/gonk
cd gonk
cargo install --path gonk
```

Add some music:

```
gonk add D:/Music
```

#### Dependencies

Debian:

```
sudo apt install libasound2-dev pkg-config build-essential
```

Fedora:
> Not tested.

```
dnf install alsa-lib-devel pkgconfig
```

Alpine:

```
apk add pkgconf alsa-lib-dev alpine-sdk
```

## ⌨️ Key Bindings

Windows: `%appdata%/gonk/gonk.toml`

Linux: `~/.config/gonk/gonk.toml`

| Command     | Key         |
|-------------|-------------|
| Move Up     | `K / UP`    |
| Move Down   | `J / Down`  |
| Move Left   | `H / Left`  |
| Move Right  | `L / Right` |
| Volume Up   | `W`         |
| Volume Up   | `S`         |
| Play/Pause  | `Space`     |
| Previous    | `A`         |
| Next        | `D`         |
| Seek -10s   | `Q`         |
| Seek 10s    | `E`         |
| Delete Song | `X`         |
| Clear Queue | `C`         |
| Change Mode | `Tab`       |
| Search      | `/`         |
| Quit        | `Ctrl+C`    |
| ?           | `Escape`    |
| ?           | `Backspace` |

`1, 2, 3` moves the queue margins forward. `Shift + 1, 2, 3` moves them backwards.

## ⚒️ Troubleshooting

If your music player has broken lines, increase your zoom level or font size.

![](media/broken.png)

## ❤️ Contributing

Feel free to open an issue or submit a pull request!
