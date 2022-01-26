<h1 align="center" style="font-size: 55px">Gronk</h1>
<!-- <p align="center" style="font-size: 30px">A simple terminal music player</p> -->

<div align="center" style="display:inline">
      <img src="media/gronk-2x.gif">
</div>

## ✨ Features
- Vim-style keybindings
- Easy to use
- Fuzzy search
- Mouse support

## 📦 Installation

> Linux and MacOS have not been testing

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

## ⚒️ Troubleshooting

> If somethings goes wrong you can always delete the database

| OS            | Path             |
|---------------|------------------|
| Windows       | %appdata%/gronk  |
| Linux & MacOS | ~/.config/gronk/ |


## TODO

- [x] Song metadata (duration)

- [x] Global hotkeys

- [ ] Allow the user to click on songs in the queue(partialy working)

- [ ] Toggles for artist/album/song only search

- [ ] Gronk player and UI should be seperate like mpd/client. The player should hold state such as the queue, volume and handle the music output.

## ❤️ Contributing

Feel free to open an issue or submit a pull request!