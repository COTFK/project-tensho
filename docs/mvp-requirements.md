# Burning Draw MVP - Fire King simulator

## Objective
Provide a cross-platform, single-player "sandbox" environment where players can practice turn 1 combos using a predefined Fire King deck.

## Jobs
- **Combo practice**: When I learn Fire King, I want to play out my hand so I can practice and memorize combos.
- **Mulligan**: When I draw a brick hand, I want to reset the game instantly so I can spend time practicing combos instead of reshuffling.

## Scope
- The environment loads a predefined Fire King deck ([Dinh-Kha Bui's French Open Montpellier 2025 winning list](https://ygoprodeck.com/deck/fire-king-591564)).
- The environment offers a fully-playable Yu-Gi-Oh! simulator, equivalent to EDOPro's Hand Test mode (no opponent, MR2020, 5 cards in starting hand).
- The environment offers a Reset button to instantly restart the duel, clear the board, and re-shuffle the deck.

## Technical Requirements
### Stack
- Framework: Dioxus 0.7 (Native)
- Logic Engine: ocgcore (C++), compiled to WASM via Emscripten
- Card scripts and database: Project Ignis Delta
- Card images: YGOPRODeck API

### Data Flow
- An async loop instantiates and manages an `ocgcore` instance via the FFI.
- `ocgcore` events are accumulated Rust-side as a global signal.
- UI is rendered as Dioxus components based on accumulated state.
- User intent is translated into messages passed to the `ocgcore`.

### State
- `InstanceID` - a newtype over ULID that identifies a card instance
- `Card` - a `struct` holding the card InstanceID, CDB card ID
- `CardData` - `struct` holding card properties from CDB (ATK, DEF, card text)
- `Action` - `enum` with all the actions a card can take as variants
- `Hand` - `Vec<Card>`; cards can be added, removed or moved; hand can be shuffled
- `Zone` - `struct` with index, zone type (`enum` of MMZ, EMZ, S/T); seems immutable for now
- `Field` - `HashMap<Zone, Option<Card>>`; cards can be added/removed from zones so values must be mutable; keys should be set once and immutable afterwards so HashMap seems fine here;
- `GY` - `Vec<Card>`; cards can be added or removed
- `Deck`, `ExtraDeck`, `Banishment` - `Vec<Card>`; cards can be added or removed; can be face-up or face-down
- `GameState` - global signal struct holding all of the above

### Duel Initialization
- On launch, spawn a coroutine that initializes `ocgcore`.
- The coroutine must load all `.cdb`s and Lua scripts required to load the predefined deck, then start a MR2020 duel, 8000LP, 5 cards in hand, with the predefined deck in player 1's control. Player 2 starts with no deck and 8000LP, and cannot lose to deck-out.
- Instantiate a global HashMap<u32, CardData> with all the cards in the deck.

### UI
- Hand: a Master Duel-like hand, with cards fanned out at the bottom of the viewport; cards are highlighted if actions are available
- Field: Just Player 1's half of the game field, similar layout to EDOPro; banishment could be moved depending on needs.
- Deck, Extra Deck, GY & Banishment - can be interacted with to see their contents (reusable `Pile` component)
- Reset button - when interacted with, restarts the game
- Card text - popup element when a card in hand, field or pile is selected

### Interaction
- Interacting with a card should store its ID in a signal, and "select" it accordingly depending on area (highlight on field, lift up in hand)
- Interacting with a card should show the currently available actions (summon, set, activate, change position, etc), and allow user to execute those actions
- Interacting with a card should show the card text in an appropriate pop-up
- Cards in hand with available actions should glow accordingly (yellow glow for activate, blue glow for summon/set; activate supersedes summon/set)
  - GameState should have an `available_actions: HashMap<InstanceID, Vec<Action>>` field; on state update, query `ocgcore` for available actions so they can be mapped properly


### Messages
Actions should send messages to the `ocgcore` coroutine; UI should wait for updated state and make the required changes.

## Definition of Done
1. The application starts a YGO simulation with a fully shuffled deck, and 5 cards drawn.
1. The player can successfully perform all the combos in the [Fire King guide](https://fire-king.arqalite.org/)
1. The UI state is always appropriately accumulated from `ocgcore` messages.
1. The player cannot perform illegal moves that `ocgcore` does not allow.