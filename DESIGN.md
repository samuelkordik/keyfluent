# DESIGN — Keyfluent

<!-- Written by ux-plan. Later skills (ux-build, ux-refine, ux-audit) read this file. Keep it short and current. -->

- **Updated:** 2026-11-25
- **Status:** chosen (Samuel, 2026-11-25)
- **Class:** native-desktop (Rust + egui 0.37)
- **House style:** none; the project's own direction below
- **Files:** `tokens.json` (default light theme + all non-color tokens), `tokens.dark.json` (default
  dark theme), `docs/design/themes/themes.json` (all 15 themes, generated and AA-checked by
  `build_themes.py`; `REPORT.md` lists every changed color), `docs/design/mockups.html` (key screens;
  screenshots in `docs/design/mockups-shots/`), history in `docs/design/directions.md`.

## 1. Product and users
A native desktop app that teaches keyboard shortcuts by doing: it shows a goal, you press the keys
in a simulated tmux, it grades correctness and speed, and FSRS schedules the reviews (spec:
`docs/superpowers/specs/2026-11-25-keyfluent-design.md`). Users: Samuel first (tmux beginner, Mac
laptop), then complete beginners. A few minutes once or twice a day, on a laptop or desktop
(minimum window 1000 × 700), keyboard first.

## 2. Primary tasks
1. Do today's practice: Today → Start → cards → session end. Daily. Done without reading anything.
2. Learn a new card (show → guided → recall). Most sessions. The beginner knows what to press.
3. Understand a miss: what went wrong and the shortest route. Every miss. No shame.
4. See progress and pick a card to practice. Weekly.
5. Set preferences (theme, fonts, sound, keys, bindings). Rarely.

## 3. Content inventory
Intent sentence · simulated terminal (cell grid: panes, borders, tmux status line, copy-mode
selection) · goal picture · keys (keycaps, tmux notation on hover) · prefix indicator · session dots
(done, current, missed, to do) · combo count · result line (stars, grade word, time vs target, New
best, Shortest route) · teach note · due and new counts · days this month · categories (Basics, Copy
mode, Sessions, Layout) · card tiles (mastery band, best stars, times, next review) · session summary
· preferences · credits.

## 4. Direction
**Arcade** — an early-90s home computer / arcade machine that happens to teach tmux: a demoscene
title screen, an arcade score line after every card, a stage-select progress grid, and a terminal
that looks like the real thing.

- Mood: playful, bold, warm.
- Layout: full-window screens, no sidebar. Today is the title screen; Practice, Progress,
  Preferences, About and Session end are full screens. A function-key bar along the bottom of every
  screen; box-titled panels (`terminal`, `goal`) in Practice; a 6 px six-color stripe on top of the
  working screens.
- Density: comfortable (base 14 pt, 16/24 pt screen padding).
- Signature moves: (1) the cracktro title screen (raster bars, sine-wave Xenon logo, starfield,
  scroller of tips and greetings); (2) the result line with stars; (3) the stage-select Progress grid;
  (4) synthesized chiptune sounds, and a dial-up modem handshake on launch.
- Trade-off accepted: a lot of motion and sound, all optional. Every animated or audible element
  has a still / silent form (reduced motion, Sound off), and none of them carries meaning alone.

**Fun, not trying too hard (spec Goal 9).** Use what helps learning and feels good: instant
feedback, stars, personal bests, combo, session summary, bonus rounds for scenarios. Never: guilt,
streaks you can lose, nagging, coins, mascots, fake urgency. The combo ends silently. Practice days
show as "days this month".

## 5. Tokens
Values live in `tokens.json` / `tokens.dark.json` and `themes.json`. Sizes are egui points.

| Area | Decision | Why |
|---|---|---|
| Themes | 15 themes; one theme colors the **whole app** (chrome + terminal). Defaults: dark **Tomorrow Night Bright**, light **Tomorrow**; Appearance: dark / light / follow system | Samuel |
| Theme set | Tomorrow ×5 (Night Bright, Night, Night Eighties, Night Blue, Tomorrow), Solarized dark/light, Gruvbox dark/light, Dracula, Nord, Catppuccin Mocha/Latte, Green and Amber phosphor (Keyfluent originals; monochrome) | Samuel; all MIT |
| Theme roles | Each theme maps 12 slots (bg, surface, raised, fg, muted, 7 accents) to roles: background, surface, surface-raised, foreground, muted, border, primary (=blue), on-*, focus (=yellow), danger/warning/success/info, plus aliases (below) | One mapping for every theme |
| AA rule | **Themes are tested content.** Text ≥ 4.5:1 (fg and muted on background and surface; accents on background; on-accent on accent); border and focus ≥ 3:1 on background and surface. Failing colors move in OKLCH lightness only (same hue and chroma). 51 colors changed across 11 themes; Night Bright, Mocha and both phosphors pass unchanged | WCAG 2.2 AA; Samuel's taste kept |
| Original colors | Preferences → Appearance → **Use original theme colors** turns the changes off (marked "may be hard to read") | Samuel |
| Aliases | category: Basics blue, Copy mode green, Sessions purple, Layout orange · grade: Easy aqua, Good green, Hard yellow, Again red · stars on yellow / off surface-raised · mastery: new muted, learning yellow, young aqua, mature green · terminal status line green / on-green · F-key caps blue / on-blue · stripe red→blue | Hue is never the only signal: words, letters and stars always present |
| Monochrome themes | Phosphor themes set every accent to the phosphor color; categories, grades and mastery read from words, letters and stars | Already the rule |
| Type | Interface and terminal: **Monaspace Argon** (both user-selectable, separately). Display: **Krypton, ALL CAPS**, wght 800, wdth 125, tracking 0.03 em. Title logo: **Xenon**, title case ("Keyfluent"), wght 800. Notes and hints: **Radon**. Sizes 12 / 14 / 17 / 21 / 27 / 34 / 42 (ratio ≈ 1.25); intent = 21 (20% under round 2's 26); screen titles 27; session end 34; logo 64 | Samuel; caps only for short display text |
| Fonts bundled | Monaspace Neon, Argon, Xenon, Radon, Krypton (v1.400, variable), Inconsolata, Source Code Pro, Fira Code — all SIL OFL 1.1 (~9.6 MB) | Samuel |
| Fonts installed | Preferences lists installed **monospace** fonts (via `fontdb`, local read only) and accepts a .ttf/.otf file (copied into the app-data folder). When the interface font isn't Monaspace, Krypton/Xenon/Radon roles fall back to it at the same weights | Samuel |
| Space / radius | 4-pt scale (4, 8, 12, 16, 24, 32). Radius 0 (square everything); 1 pt borders; 2 pt focus ring | Arcade / text-mode look |
| Motion | Expressive on Today and Session end; feedback only in Practice (score count-up 350 ms, New best pulse, result hold 1 s). **Reduced motion** (OS setting or Preferences): everything still | Spec §8 |
| Sound | Synthesized in code (square, triangle, noise; no recordings). See §8 | Samuel |
| Charts | Progress charts (calendar, forecast, times) use the mastery and grade aliases; checked per theme when built (`dataviz` rules) | — |
| Dark mode | Two theme slots (dark, light) + follow system | — |

## 6. Layout and responsive behavior
Desktop only. Minimum window 1000 × 700; designed there first. Wider: the practice terminal grows
(goal panel fixed at 190 pt); Today and Session end stay centered; the stage grid wraps tiles.
Taller: the terminal grows. In-app zoom `Cmd/Ctrl +/-` (outside practice) scales all tokens.

## 7. Key screens
Mockups at 1000 × 700: `docs/design/mockups.html`.

### Launch
Black window, a dial-up modem handshake (~3 s; DTMF dial, answer tone, carrier "scramble") while
`CONNECT 33600` types out, then the title screen fades in. Any key skips. Sound off or launch sound
off: no screen, straight to Today.

### Today (title screen)
```
 ═════ raster bars (red, orange, yellow) ═════
               K e y f l u e n t            ← Xenon 64, sine wave
   tmux · prefix [Ctrl]+[b] · 12 reviews due · 5 new cards
               [▶ Start practice]           ← focus, Enter
                  Progress
                  Preferences
              8 days this month
 ═════ raster bars (aqua, blue) ═════
 … TIP: PRESS CTRL-B, LET GO, THEN % … GREETINGS TO …   ← scroller, Krypton
 [⏎] Start  [F3] Progress  [F4] Preferences  [F1] About          [F8] Sound on
```
- Primary action: Start practice (Enter). Up/Down move; menu ticks play.
- States: nothing due and no new cards ("All done for today. Next review: tomorrow, 4 cards." Start
  becomes "Practice anyway"); first launch (the intro replaces the menu: what tmux is, what the prefix
  is, then straight into card 1); custom prefix ("Your prefix is Ctrl-a", spec §6.7); reduced motion
  (bars still, logo flat, scroller replaced by one static tip line); window unfocused (animation pauses).
- Scroller content: rotating tips from the cards' `teach` text, then greetings to the theme and
  font authors (the About credits).

### Practice
```
 ▀▀▀▀▀▀▀▀ stripe ▀▀▀▀▀▀▀▀
 [BASICS] new card · watch, then do it                                 combo ×4
 SPLIT THIS PANE SIDE BY SIDE                                    ← Krypton 21 caps
 ┌─ terminal ─────────────────────────────────────┐ ┌─ goal ─────────┐
 │ you@laptop:~/site$ git status                  │ │ ┌──────┬──────┐│
 │ …                                              │ │ └──────┴──────┘│
 │                                                │ │ 2 panes, side  │
 │ [0] 0:bash*                "laptop" 14:02 …    │ │ Radon teach    │
 └────────────────────────────────────────────────┘ └────────────────┘
 keys [Ctrl]+[b] ◆ prefix on                          ●●●○○○○○○○○○○○○○○
 [⌃⇧P] Pause   Show me (in the pause menu)            every other key goes to tmux
```
- No numbers row (card/your best/target hidden; the dots show position).
- Result line (replaces nothing; appears above the keys row): `★★☆ GOOD 1.6 s · target 2.0 s
  [NEW BEST]  Enter: next card`; on a longer route adds `Shortest: [Ctrl]+[b] [%]`. Auto-advance
  after 1 s or Enter.
- Miss: `☆☆☆ AGAIN · here's how` → the start state resets, keys are shown and animated, one ungraded
  retry; the dot turns red. No sad sound, just a soft low tone.
- States: introduction (show → guided → recall, teach note in Radon, keys visible in the goal panel);
  wild mode (goal panel shows only "?" and "from memory"); prefix pending; command prompt / copy mode
  (the terminal shows them as tmux does); OS-reserved key badge.
- Pause menu (pause chord): Resume, Show me, Skip card, End session. A centered box-titled panel.

### Progress (stage select)
Rows per category (label in the category color, Krypton caps), a 40 pt tile per card: `★` mature,
`Y` young, `L` learning, `·` new, dashed `?` bonus round (scenario; unlocks when its cards are
solid). Arrows move; the selected tile's detail panel shows best stars, best time vs target, last 10
times (bars), keys, reviews, next review. Enter practices that card. Tab switches to Calendar (30
days) and Forecast (14 days). Empty state: all tiles `·` and "Start practice to fill this in."

### Session end
Starfield; the summary scrolls up and stops (still with reduced motion): SESSION COMPLETE · cards,
time, best combo · count per grade with stars · New bests (card, time) · "Next: 4 reviews tomorrow · 2
new cards waiting" · Press any key. A short jingle plays.

### Preferences
Full screen, sections as box-titled panels, all keyboard-operable: **Learning** (daily new cards,
wild mode, speed expectations, scenario unlock), **Keys** (copy-mode keys, Mac Option as Meta, pause
chord), **Bindings** (stock / import), **Appearance** (follow system / dark / light; dark theme; light
theme — each a list with a live preview of the practice screen; Use original theme colors; terminal
font; interface font; size; reduced motion: follow system / on / off), **Sound** (Sound on/off,
volume, launch modem sound on/off, key clicks off by default), **Data** (export, import, reset).

### About (F1)
Version, MIT license, one-line description; box-titled panels: Themes (name, author, license, link —
generated from `themes.json`, with a note that some colors are adjusted), Fonts (Monaspace — GitHub
Next; Inconsolata — Raph Levien; Source Code Pro — Adobe; Fira Code — Nikita Prokopov; all OFL),
Sounds (all synthesized by Keyfluent), Software (Rust, egui, rs-fsrs, full open-source license list).

## 8. Components and states
- **F-key bar:** surface strip, keycaps blue/on-blue; shows only keys that work on that screen. In
  practice it shows the pause chord only (every other key belongs to tmux).
- **Box-titled panel:** 1 pt border role, title inset in the top border (`─ terminal ─`).
- **Keycap:** foreground fill, background text; tmux notation in a tooltip on hover.
- **Menu item:** inverse bar (foreground fill) when focused; Enter activates.
- **Tile:** 40 pt square; filled by mastery band with a letter/★; focused = 2 pt foreground outline.
- **Stars:** 0–3 = Again / Hard / Good / Easy; off stars in surface-raised; always next to the grade word.
- **Session dots:** done (green), missed (red), current (foreground), to do (surface-raised).
- **Result line:** green 1 pt frame on success, red on a miss; grade word in Krypton caps.
- **Focus:** 2 pt focus-role ring on every focusable element outside practice.
- **Sounds** (synthesized; volume one setting; all off with Sound off; F8 toggles on Today):
  menu move tick · menu select · correct (pitch rises with the grade: Hard < Good < Easy) · New best
  fanfare (short arpeggio) · combo every 5 (rising arpeggio) · miss (soft low two-note, not a buzzer)
  · scenario unlocked · session end jingle · launch modem handshake. Sounds never carry meaning
  alone. Default: **on**, including the modem at first launch; the launch screen says
  "Any key: skip · Sound can be turned off in Preferences" (Samuel, 2026-11-25).

## 9. Voice and copy
- Terms: **card** (not drill/exercise), **session**, **review**, **new card**, **prefix**, **combo**,
  **New best**, **bonus round** (= scenario, in the UI), **Show me**, grades Easy / Good / Hard / Again.
- Tone: short, warm, plain, a little playful in display text (READY., SESSION COMPLETE, NEW BEST).
  Card text in sentence case in the data; ALL CAPS only as the Krypton display style.
- Misses: say what happened and show the way; never blame ("Not quite. Here's how:").

## 10. Platform and accessibility rules
- WCAG 2.2 AA in every shipped theme (tested); original-colors mode labeled as possibly hard to read.
- Color is never the only signal (words, letters, stars, icons).
- Reduced motion: no waves, rolls, scrolls, sweeps or count-ups; no flashing anywhere (nothing
  flashes more than 3 times a second even with motion on).
- Sound is never the only signal; captions not needed because every sound has a visual twin.
- Keyboard: everything outside practice is keyboard-operable with a visible focus ring; F-keys are
  shortcuts, not the only way (menus and Tab work too). On a Mac, F-keys may need Fn: every F-key
  action also has a letter shortcut shown in the menu (e.g. P Progress, comma Preferences).
- AccessKit labels for custom widgets (tiles, stars, keycaps, terminal summary).
- Text scales with the OS and the in-app zoom. The terminal cell grid scales as a whole.

## 11. Decisions and rejected options
| Date | Decision | Options considered | Why this one | Revisit if |
|---|---|---|---|---|
| 2026-11-25 | Direction C "Boot screen" over A "Mainframe" and B "Workstation" | A, B, C | Home computer / arcade energy is the point; Monaspace mix is fun | — |
| 2026-11-25 | From A: F-key bar, box-titled panels, progress dots | — | Terminal feel at low cost | — |
| 2026-11-25 | Round 2: base = 1 Arcade cabinet (title −20%, HUD numbers hidden); Progress = 3's stage select; stars from 3; Today = 4's cracktro with a Xenon title-case logo; session end = 4's starfield | 1, 2 Home computer (window border flashes), 3 Cartridge, 4 Cracktro | Samuel | — |
| 2026-11-25 | Rejected: 2's flashing window border | — | Not chosen | Wanted as an option later |
| 2026-11-25 | Rejected: scanline / CRT effect | — | Samuel: doesn't work here | — |
| 2026-11-25 | Rejected: sentence-case Krypton display text | — | Looked wrong; ALL CAPS Krypton is right | — |
| 2026-11-25 | Chiptune sounds + modem launch sound, synthesized | Recorded samples | No licensing questions; tiny; original | — |
| 2026-11-25 | Themes adjusted to AA by default, originals as an option | Ship originals only; drop failing themes | Keeps favorites, keeps everyone able to read | — |
| 2026-11-25 | Sound on by default, modem at first launch with a "turn off in Preferences" note | Off until enabled | Samuel | Users find it jarring |
| 2026-11-25 | Practice, Progress, Session end, About approved as mocked; Today approved except the logo font | — | Samuel | Logo font at brand-identity |
| 2026-11-25 | Spec: fun / game-like feedback is a goal; only "trying to be a game" is an anti-goal | — | Samuel | — |

## 12. Open questions
1. **Title logo font:** Xenon title case still isn't right (Samuel, 2026-11-25). Revisit with
   `ui-ux:brand-identity` (P7, icon and wordmark). Build with Xenon until then.
2. **Audio crate:** `rodio`/`cpal` (MIT/Apache) need a `cargo deny` license and no-network check, and
   ALSA headers in the dev container. Check at P3 start.
3. **Links in About:** opening a link in the browser goes through the OS, not Keyfluent's network,
   but the "no network" promise is simpler with **Copy link** only. Proposed: Copy link.
4. **Texture healing / ligatures:** egui 0.37 shapes text with harfrust and supports variable axes;
   confirm Monaspace texture healing (and Fira Code ligatures, which should be **off** in the
   terminal) in the first snapshot test.
5. **Binary size:** bundled fonts add ~9.6 MB. Fine for a desktop app; could subset later.
6. Copy for the first-launch intro and the scroller tips (written with P3/P5).
