<h1 align="center" style="font-size: 55px">Gronk</h1>
<p align="center" style="font-size: 30px">⚠️ This is a work in progress.</p>

<div align="center" style="display:inline">
      <img src="media/gronk-2x.gif">
</div>

## ✨ Features
- Vim-style key bindings
- Easy to use
- Fuzzy search
- Mouse support
- Cross-platform

## 📦 Installation

> MacOS has not been testing

Debian requires `libasound2-dev`

Fedora requires `alsa-lib-devel`

#### From source

```
git clone https://github.com/zX3no/gronk
cd gronk
cargo install --path gronk
```

Then add some music:
```
gronk add D:/Music
```

## ⌨️ Key Bindings

| Command     | Key       |
|-------------|-----------|
| Move Up     | K / UP    |
| Move Down   | J / Down  |
| Move Left   | H / Left  |
| Move Right  | L / Right |
| Volume Up   | W         |
| Volume Up   | S         |
| Play/Pause  | Space     |
| Previous    | A         |
| Next        | D         |
| Seek -10s   | Q         |
| Seek 10s    | E         |
| Clear Queue | C         |
| Change Mode | Tab       |
| Search      | /         |
| ?           | Escape    |
| ?           | Backspace |


## ⚒️ Troubleshooting
If somethings goes wrong with the database, you can always delete it here:

| OS            | Path             |
|---------------|------------------|
| Windows       | %appdata%/gronk  |
| Linux & MacOS | ~/.config/gronk/ |

If your music player has broken lines, just increase your zoom level or font size.

![](media/broken.png)


## TODO
- [x] Many files types fail to play correctly

- [x] Song metadata (duration)

- [x] Global hotkeys

- [ ] Allow the user to click on songs in the queue(partialy working)

- [ ] Toggles for artist/album/song only search

- [ ] Gronk player and UI should be seperate like mpd/client. The player should hold state such as the queue, volume and handle the music output.

## ❤️ Contributing

Feel free to open an issue or submit a pull request!