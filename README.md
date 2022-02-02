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

Debian requires `libasound2-dev`

Fedora requires `alsa-lib-devel`

#### From source

```
git clone https://github.com/zX3no/gonk
cd gonk
cargo install --path gonk
```

Then add some music:

```
gonk add D:/Music
```

## ⌨️ Key Bindings

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
| ?           | `Escape`    |
| ?           | `Backspace` |

`1, 2, 3` moves the queue margins forward. `Shift + 1, 2, 3` moves them backwards.

## ⚒️ Troubleshooting

If somethings goes wrong with the database, you can always delete it using `gonk reset`.

If your music player has broken lines, increase your zoom level or font size.

![](media/broken.png)

## TODO

- [ ] Songs do not play while the volume is 0

- [ ] Configuration file for key bindings

- [x] Fix M4A files 

- [x] Song metadata (duration)

- [x] Global hotkeys

- [ ] Settings menu (output device, music directories)

- [ ] Replay gain support

- [ ] Automatically adjust margin based on viewable songs

- [ ] Automatic updates

- [ ] Icon

- [ ] Allow the user to click on songs in the queue(partialy working)

- [ ] Toggles for artist/album/song only search

- [ ] Gonk player and UI should be seperate like mpd and ncmpcpp. The player should control the queue, volume and output.

> Should the player remember what was in the queue? I like being able to close the player and still listen.

## ❤️ Contributing

Feel free to open an issue or submit a pull request!
