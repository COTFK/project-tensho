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

### Linux users
**These steps were tested on a fresh Fedora 44 install** - other distros may require different steps/commands.

#### Native build
- Install the following prerequisites: `sudo dnf install gcc gcc-c++ webkit2gtk4.1-devel openssl-devel libxdo-devel`
    - For other distros, see the [Tauri docs](https://tauri.app/start/prerequisites/#linux).
- Run `dx serve --desktop`

#### Build for the web
- Make sure you have Emscripten 6.0.0 installed; for best results, use the helper script at `scripts/install-emsdk.sh`.
- Make sure GCC is installed: `sudo dnf install gcc`
- Dioxus CLI should handle everything for you afterwards. Run `dx serve --web` and you're good to go!

### Windows/macOS
Windows and macOS should be theoretically supported, but have not been tested yet. **If you manage to get them running, raise a pull request!**