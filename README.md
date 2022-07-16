<h1 align="center" style="font-size: 55px">Gonk</h1>

<h3 align="center">A terminal music player.</h3>

<div align="center" style="display:inline">
      <img src="media/gonk.gif">
</div>

## ⚠️ Warning

- Gonk is under heavy development. Expect breaking changes.

## ✨ Features
- Easy to use
- Cross-platform
- Plays FLAC, MP3, OGG, M4A and WAV
- Fuzzy search
- Vim-style key bindings
- Mouse support

## 📦 Installation

> I recommend using a font with ligatures for the best experience.

#### Dependencies

Debian:

```
sudo apt install libasound2-dev pkg-config build-essential libjack-jackd2-dev
```

Alpine:

```
apk add pkgconf alsa-lib-dev alpine-sdk
```

Arch:

```
pacman -S pkgconf alsa-lib base-devel
```

#### crates.io

```
cargo install gonk
```

Add your music:

```
gonk add D:/Music
```

## ⌨️ Key Bindings

| Command              | Key             |
|----------------------|-----------------|
| Move Up              | `K / Up`        |
| Move Down            | `J / Down`      |
| Move Left            | `H / Left`      |
| Move Right           | `L / Right`     |
| Volume Up            | `W`             |
| Volume Up            | `S`             |
| Play/Pause           | `Space`         |
| Previous             | `A`             |
| Next                 | `D`             |
| Seek -10s            | `Q`             |
| Seek 10s             | `E`             |
| Delete Song          | `X`             |
| Clear Queue          | `C`             |
| Clear Except Playing | `Shift + C`     |
| Change Mode          | `Tab`           |
| Search               | `/`             |
| Settings             | `.`             |
| Playlists            | `,`             |
| Add to playlist      | `Shift + Enter` |
| Update Database      | `U`             |
| Quit                 | `Ctrl + C`      |
| Move Song            | `1 / Shift + 1` |
| Move Album           | `2 / Shift + 2` |
| Move Artist          | `3 / Shift + 3` |

## ⚒️ Troubleshooting

- Gonk doesn't start after an update.

  Run `gonk reset` to reset your database.

- If your music player has broken lines, increase your zoom level or font size.

  ![](media/broken.png)

## ❤️ Contributing

Feel free to open an issue or submit a pull request!
