# Setback — UI Prototype

A static HTML/CSS(/vanilla JS) UI prototype for the Setback card game, themed
as a classic wood-and-felt card table (deep baize green, dark walnut, brass
foil accents). Open `index.html` in a browser to try it — no build step or
server required.

## Structure

```
index.html              Main menu (entry point)
css/
  variables.css          Design tokens: color, type, spacing, scale
  base.css                Resets, felt background, global typography
  layout.css               Shared layout: top bar, logo mark, option cards
js/
  app.js                    Generic, attribute-driven behavior shared by
                             every screen: modals, tabs, sliders, keybind
                             capture, toasts, option-card selection
components/                Each reusable piece gets its own folder with
  button/                  a standalone .css and a standalone .html demo,
  panel/                   exactly as it's used (copy-pasted) in the real
  modal/                   screens below. Treat each folder as the HTML/CSS
  tabs/                    equivalent of a component you'll port to Dioxus.
  toggle/
  slider/
  text-input/
  dropdown/
  keybind-row/
  player-slot/              (list-row style — kept for reference, unused now)
  player-node/               The circular seat card used in the lobby ellipse
  game-list-item/
  toast/
  card-chip/
  info-tooltip/               Hoverable "i" icon + popover, CSS-only
screens/
  multiplayer.html          Host or Join
  host-game.html             Public/private toggle, password if private
  join.html                   Public (→ browse) or Private (code+password)
  browse-games.html            Public table browser with search/filters
  lobby.html                Unified table setup — see below
  settings.html            Video / Audio / Keybinds / Account tabs
  game.html                Placeholder gameplay table
```

## Navigation flow (matches the spec)

Main Menu
├─ Singleplayer → lobby.html?context=single&role=host → **Start Game** → game.html
├─ Multiplayer → multiplayer.html
│   ├─ Host → host-game.html (Public → Start Table; Private → password → Start Table)
│   │                                                          → lobby.html?context=multi&role=host
│   └─ Join → join.html
│       ├─ Public → browse-games.html → Join → lobby.html?context=multi&role=guest
│       └─ Private → code + password inline → success → lobby.html?context=multi&role=guest
│                                            → failure → toast alert, stays put
│              (demo credentials: code `DEMO1`, password `setback`)
├─ Settings → settings.html (tabs: Video / Audio / Keybinds / Account)
└─ Exit Game → no-op, per spec

Every sub-screen has a **back button pinned to the same top-left spot**
(`.back-button` in `css/layout.css`) so the mouse never has to hunt for it.
Singleplayer and Multiplayer are now equally emphasized on the main menu.

## The lobby, in more detail

One screen (`lobby.html`) now serves singleplayer, host, and guest — the
`context` and `role` query params (set by whichever screen links to it)
decide what's shown and what's editable:

- **Table Rules** (left panel): Teams Rule Set, Win By 2, Negative Scores,
  Dealer Override, Play To — each with an info icon that shows a tooltip on
  hover/focus. Wrapped in a `<fieldset>` so disabling it for guests is one
  line (`fieldset.disabled = true`), no per-control bookkeeping needed.
- **Seats** (right, in an ellipse): 4–8 player nodes positioned with simple
  trigonometry (`css left/top` as percentages, recalculated only when the
  seat count changes — resizing the window is free, no JS needed). Host-only
  controls: **+ Add Player**, a **×** to remove a seat (min 4, can't remove
  yourself), and **click two seats to swap** them (turn order). When Teams
  Rule Set is Custom, the team badge on each seat is also clickable to
  reassign it.
- A small **Preview: Host / Guest** selector in the top-right lets you flip
  between the editable and view-only states without leaving the page — it's
  a review aid for this prototype, not part of the real game (I'd wire the
  real value from session/auth state instead).

## Notable interactions that already work

- **UI scale**: the slider in Settings → Video writes a `--ui-scale` custom
  property onto `<html>`, which drives the root `font-size`. Since the whole
  system is sized in `rem`, this scales everything uniformly.
- **Keybind rebinding**: click a keycap, press any key, it updates. Reset
  button restores the default shown in the row's `data-default` attribute.
- **Settings tabs** now sit inside a fixed-height, internally-scrolling body
  (`.tabs__body`), so the panel doesn't resize as you switch tabs.
- **Lobby**: add/remove/swap seats, toggle Host/Guest preview, team-badge
  reassignment under Custom rules, all of Table Rules with tooltips.
- **Volume sliders**, **fullscreen/windowed toggle label**, **host
  public/private toggle** (private's password field only exists in the DOM
  once you pick Private — nothing to hide for Public), and the **join
  failure toast** are all functional in the browser.
- Everything is driven by small, reusable CSS classes (`.btn--primary
  .btn--lg`, `.option-card`, `.player-node.is-selected`, etc.) rather than
  one-off styles, so variations are just class combinations.

## Design notes / open questions for you

- I used vanilla multi-page navigation (`<a href="...">`) rather than a
  single-page JS router, since it's the simplest thing that reliably works
  offline and translates cleanly to Dioxus routes/components later. Happy to
  convert to a single `index.html` with JS-toggled screens if you'd rather
  prototype it that way.
- Placeholder content (player names, table codes, hands) is illustrative;
  swap freely once real data is wired up.
- Fonts are loaded from Google Fonts (Playfair Display, Poppins, IBM Plex
  Mono) via `@import` in `base.css` — swap for local files if you need this
  fully offline.
- `player-slot.css/html` (the old list-row seat style) is no longer used by
  any screen now that the lobby is ellipse-based — left in `components/` in
  case it's useful elsewhere (e.g. an in-game scoreboard), otherwise safe to
  delete.
