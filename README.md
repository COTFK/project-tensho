# Project Tensho

A single-player Yu-Gi-Oh! practice environment based on [`edo9300/ygopro-core`] (the core that powers [Project Ignis: EDOPro](https://projectignis.github.io/)).

You can play the web version here: https://tensho.arqalite.org/

[`edo9300/ygopro-core`]: https://github.com/edo9300/ygopro-core

## Status

**Tensho is currently in early development - expect instability.**

The duel starts with a predefined Fire King deck ([Dinh-Kha Bui's French Open Montpellier 2025 winning list](https://ygoprodeck.com/deck/fire-king-591564)) and currently you cannot change decks.

You can play the first turn of the duel, Main Phase 1 only. There is no way to change phases at the moment.

All combos in [The Fire King Sanctuary] can be successfully performed.

[The Fire King Sanctuary]: https://fire-king.arqalite.org/

## Build
### Requirements
- [**Rust**] 1.95+ (earlier versions may or may not work)
- [**Dioxus CLI**]
- [**Emscripten SDK**] 6.0.0 (_optional_; for `wasm32-unknown-unknown` targets)
    - **6.0.1 currently does not work!** 

[**Rust**]: https://rust-lang.org/
[**Dioxus CLI**]: https://dioxuslabs.com/learn/0.7/getting_started/#install-the-dioxus-cli
[**Emscripten SDK**]: https://emscripten.org/

### Web
**These steps were tested on a fresh Fedora 44 install** - other distros/OSes may require different steps/commands.

- Make sure you have Emscripten 6.0.0 installed; for best results, Linux users can use the helper script at `scripts/install-emsdk.sh`.
- Make sure a C compiler is installed: `sudo dnf install gcc`
- Dioxus CLI should handle everything for you afterwards. Run `dx serve --web` and you're good to go!

### Linux
**These steps were tested on a fresh Fedora 44 install** - other distros may require different steps/commands.

- Install the following prerequisites: `sudo dnf install gcc gcc-c++ webkit2gtk4.1-devel openssl-devel libxdo-devel`
    - For other distros, see the [Tauri docs](https://tauri.app/start/prerequisites/#linux).
- Run `dx serve --desktop`

### Windows
On Windows, `dx serve --desktop` seems to work out of the box.

### macOS
macOS should be theoretically supported, but has not been tested yet. **If you manage to get it running, raise a pull request!**

## Contributing
We accept contributions! Reach out to us in the [Circle of the Fire Kings Discord server](https://discord.gg/8JtxHUAdGq) to chat and discuss your proposed changes!

## License
Project Tensho is licensed under [the AGPL-3.0 license](LICENSE).

Unless explicitly stated otherwise, any contribution to this project shall be licensed under the aforementioned license, without any additional terms or conditions.

## Credits

Icons are provided by [Ikonate] & [RPG-Awesome].
Card images courtesy of [YGOPRODeck].
Game core & card data provided by [Project Ignis].

[Ikonate]: https://ikonate.com/
[RPG-Awesome]: https://nagoshiashumari.github.io/Rpg-Awesome/
[YGOPRODeck]: https://ygoprodeck.com/api-guide/
[Project Ignis]: https://github.com/ProjectIgnis