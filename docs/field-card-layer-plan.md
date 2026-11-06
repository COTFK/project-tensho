# Field and card layer transition plan

## Strategy

Keep the game playable after every commit. Prepare replacement code alongside
the current UI, use a temporary opt-in mode for integration, and switch the
default only once the replacement has passed its regression gate. Each task
includes focused tests and proposes one independently deployable commit.

Architectural boundaries:

- The upstream core remains unchanged and authoritative.
- The card layer assigns instance IDs and updates its list from ordered
  messages; it is a presentation of the core state, not another authority.
- The displayed list may temporarily lag behind the core during playback.
- Prompts and actions are enabled only when presentation has caught up.
- Modals remain selection and preview views, not owners of moving board cards.
- Hidden shuffles are identity boundaries; do not infer identity from card code.

Dependencies below are prerequisites, not a requirement that every task wait
for the preceding number.

## A. Build and test tracking without changing gameplay

The existing UI remains active throughout this phase.

### 1. Extract query decoding

Separate byte decoding from FFI calls, preserving existing query results.

- Test: occupied and empty zones, malformed records, card properties, and
  sequence ordering.
- Commit: `refactor(core): isolate query decoding`

### 2. Prepare the persistent card list

Prepare one list of cards for the future card layer, with frontend instance
IDs, locations, and address lookup. Provide an initializer for the ordered
decks produced by duel setup, without wiring it into gameplay yet. Reuse
existing player and location types and preserve full-width sequences. IDs must
not depend on card code or position. Keep the existing UI active.

- Test: initialization and ordering, custom hands, identical copies, address
  uniqueness, allocation, and reset.
- Commit: `feat(ui): prepare persistent card list`

### 3. Track moves, draws, and card lifetimes

Update the card list directly from ordinary movement and draw messages,
including sequence shifts, creation, and disappearance. Produce ordered
events containing enough historical data for later playback. Harden the draw
decoder as part of making it a reliable input.

- Depends on: 2.
- Test: duplicate copies, multiple draws, same-pile moves, creation and
  removal, and truncated draw messages.
- Commit: `feat(tracker): track card movement and draws`

### 4. Track overlay relationships

Handle attaching and detaching materials, material reindexing, and moving hosts
without replacing material identities.

- Depends on: 3.
- Test: multiple materials, host movement, detachment, and host removal.
- Commit: `feat(tracker): track overlay card lifetimes`

### 5. Track position and appearance changes

Decode missing position and flip notifications and update
presentation-relevant state without changing IDs. Preserve concealment rules.

- Depends on: 2.
- Test: attack and defense rotation, setting and revealing, hidden codes, and
  unchanged identity.
- Commit: `feat(tracker): track card position changes`

### 6. Track atomic swaps

Decode swaps and update both addresses atomically, avoiding temporary
collisions.

- Depends on: 3.
- Test: occupied-zone swaps, controller changes, and associated material
  addresses.
- Commit: `feat(tracker): track atomic card swaps`

### 7. Track bulk card removal

Decode bulk removal messages and clean up the card list and relationships.

- Depends on: 4.
- Test: multiple removals, sequence shifts, and dangling overlay references.
- Commit: `feat(tracker): track bulk card removal`

### 8. Handle hidden shuffles

Cover hand, deck, and extra-deck shuffles. Explicitly invalidate
correspondence where the protocol hides it; do not guess identities from
duplicate codes.

- Depends on: 3.
- Test: identical copies, hidden cards, selection invalidation, and card-count
  preservation.
- Commit: `feat(tracker): handle hidden shuffle boundaries`

### 9. Handle set-card shuffles

Decode the supplied relocation information and distinguish known
correspondence from intentionally concealed correspondence.

- Depends on: 4, 8.
- Test: occupied zones, hidden destinations, overlays, and unaffected
  identities.
- Commit: `feat(tracker): track set-card shuffles`

### 10. Track deck reversal and reveals

Handle deck ordering and orientation and top-card reveal notifications without
treating every reveal as a new card.

- Depends on: 3.
- Test: reversed draws, top offsets, and hidden and revealed appearance.
- Commit: `feat(tracker): track deck order and reveals`

### 11. Track graveyard-deck exchanges

Implement the bulk exchange, including cards redirected to the extra deck and
any ensuing shuffle boundary.

- Depends on: 8, 10.
- Test: ordering, counts, extra-deck redirection, and duplicate copies.
- Commit: `feat(tracker): track graveyard-deck exchanges`

## B. Integrate the model while preserving the old UI

### 12. Run the card list alongside existing state

Initialize the card list directly from the actual shuffled decks and custom
hand setup before processing begins, and hold it in a UI state signal. Update
that same list from the game loop, preserving message order and existing
process-to-input behavior. Apply every structural message from startup onward.
Audit reachable structural messages and check list invariants; report detected
divergence and stop card playback instead of silently continuing with invalid
identities. Keep the existing UI and its responses unchanged.

- Depends on: 2, 3-11.
- Test: native-core startup, summon/set/move sequences, batched events,
  cards created during processing, restart, invariant failures, and unchanged
  responses.
- Commit: `feat(state): run card tracking alongside the UI`

### 13. Add card playback

Build a playback queue that applies ordered events to the same card list.
Preserve intermediate moves and appearance changes even when the core processes
several events together; do not introduce a second card identity model.

- Depends on: 3-11.
- Test: consecutive movements of one card, simultaneous groups, immediate
  playback, cancellation, and completion.
- Commit: `feat(animation): add ordered card playback`

### 14. Decouple card behavior from zone ownership

Extract reusable actions, highlights, menus, and stat decorations from field
and hand cards. Keep the existing components using those extracted pieces.
Preserve response indices and resolve actions against current logical
addresses.

- Test: summon, set, activation, chains, tributes, selection, action
  suppression, and unchanged appearance.
- Commit: `refactor(ui): decouple card actions from zones`

## C. Assemble the replacement in an opt-in mode

The default game remains unchanged. Each integration commit also keeps the
opt-in mode playable.

### 15. Add shared layout anchors

Introduce one coordinate system spanning field, hand, and piles. Keep
responsive DOM layout; use Dioxus measurement APIs where necessary.

- Test: zone alignment, aspect ratio, resize and fullscreen, and defense-card
  centering.
- Commit: `feat(ui): add shared card layout anchors`

### 16. Add the persistent card layer

Render one flat list keyed by frontend instance ID. Separate movement transforms
from rotation and hover transforms. Keep pile-resident instances available
without exposing concealed faces.

- Depends on: 13, 15.
- Test: stable DOM identity, property updates without remounting, pointer
  pass-through, and stacking.
- Commit: `feat(ui): add persistent card rendering`

### 17. Enable a layered-field preview

Add the temporary opt-in mode and replace only its field rendering. Reuse
existing placement handling, menus, and decorations. Keep its hand unchanged for
now; cross-surface animations remain disabled.

- Depends on: 12, 14, 16.
- Test: normal and extra monster zones, spell/trap zones, placement clicks,
  card actions, and legacy mode regression.
- Commit: `feat(ui): add an opt-in layered field`

### 18. Move the hand into the shared layer

Preserve the fan, selection lift, overlap, and menus. Hand-to-field movement can
now retain the same rendered instance.

- Depends on: 17.
- Test: hand resizing and reindexing, duplicate cards, summon and set
  interactions, and DOM identity across hand-field movement.
- Commit: `feat(ui): move hand cards into the shared layer`

### 19. Integrate piles and materials

Represent deck, extra deck, graveyard, banishment, and material destinations in
the shared layer. Preserve pile clicks and existing modal behavior; avoid
simultaneously drawing a moving card in its destination pile.

- Depends on: 4, 18.
- Test: draw, discard, banish, special summon, attachment and detachment, stack
  counts, and modal actions.
- Commit: `feat(ui): integrate piles into the card layer`

## D. Animate, harden, and switch defaults

### 20. Connect playback to card transforms

Enable movement and position animation in the opt-in mode. Gate prompts and
actions until the displayed state catches up, while retaining the current
core-processing behavior.

- Depends on: 12, 13, 19.
- Test: exact source and destination alignment, intermediate movements,
  simultaneous movements, no remounts, and no premature or duplicate
  responses.
- Commit: `feat(animation): animate tracked card events`

### 21. Handle interrupted and reduced motion

Wire restart cancellation, stale-callback protection, resize handling, and
reduced-motion playback. Report detected tracking divergence and stop affected
playback and interactions; require a duel restart when safe recovery is not
possible. Do not claim to detect silent divergence without a comparison.

- Depends on: 20.
- Test: restart mid-animation, resize mid-animation, reduced motion,
  interrupted transitions, and injected tracking divergence.
- Commit: `fix(animation): handle interrupted card playback`

### 22. Make layered rendering the default

Make a small wiring-only switch in the duel screen after the complete
regression gate passes. Retain explicit access to legacy mode temporarily.

- Depends on: 17-21.
- Test: existing playable workflows, custom hands, restart, fullscreen, modals,
  touch and pointer interaction, and deployed web build.
- Commit: `feat(ui): enable layered rendering by default`

### 23. Remove obsolete rendering

After confirming the default deployment, remove the legacy field and hand
rendering and temporary mode switch. Retain shared prompt and state machinery
that still serves the new UI.

- Depends on: 22 and deployment verification.
- Test: regression suite and production bundle without legacy components.
- Commit: `refactor(ui): remove legacy card rendering`

### 24. Consolidate card UI state

After legacy rendering is removed, move the card-list types and operations into
the UI state implementation and remove the temporary standalone data module.
Remove obsolete zone-specific state that no longer serves prompts or modals;
keep the rendering components separate.

- Depends on: 23.
- Test: card updates and stable keys, restart and reset, prompts and modals,
  and existing playable workflows.
- Commit: `refactor(ui): consolidate card state`

## Validation and scope

- Every commit includes focused tests; testing is not deferred to cutover.
- Reuse existing Cargo tests and native-core fixtures, plus Dioxus
  mount/identity tests.
- UI integration gates require a user-started server and user-triggered
  rebuild; browser checks must not refresh.
- No core/FFI modifications, unrelated battle animations, opponent UI, or
  tag/replay features.
- Do not stage or commit changes without separate authorization.

## Setup trade-offs and future synchronization

Direct initialization avoids a separate setup model and its duplicated core
placement logic. In return, it does not independently verify the core's
initial state; unusual redirects or unreported setup changes may leave the
card list out of sync. Test the current deck setup and its ordering, but defer
general setup validation until needed.

The card list assumes scripts do not silently create or move cards before
message processing. Core scripts can make such changes (for example, debug
scripts can use `Debug.AddCard`). This is not a known behavior of the current
setup.

Message tracking and invariants also cannot detect every silent divergence.
Snapshots are outside this transition; consider snapshot-based verification
and recovery in a future update only if synchronization issues cannot
otherwise be resolved.
