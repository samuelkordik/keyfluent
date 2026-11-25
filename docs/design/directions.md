# Keyfluent: UX brief and design directions

_2026-11-25. Stage: `ui-ux:ux-plan`, checkpoint. Samuel picks a direction; then `DESIGN.md` +
`tokens.json` get written. Visual sheet: [`directions.html`](directions.html) (screenshot in
`directions-shots/light-1440.png`; it needs the local Monaspace copies in `fonts/`, which are
not committed)._

## 1. Brief

- **Product:** a native desktop app (Rust + egui; macOS, Windows, Linux) that teaches keyboard
  shortcuts by doing: it shows a goal, you press keys in a simulated tmux, and it grades your
  speed and schedules reviews with FSRS. Spec: `docs/superpowers/specs/2026-11-25-keyfluent-design.md`.
- **Class:** `native-desktop`. **House style:** the project's own brief (below); Samuel's house
  style is not applied.
- **Users and devices:** Samuel first (tmux beginner, Mac laptop, SSH to trantor), then complete
  beginners in general. Laptop and desktop screens, minimum window 1000 × 700. Keyboard first.
- **Top tasks (ranked):** 1. do today's practice (Today → Start → cards → done);
  2. learn a new card (show → guided → recall); 3. understand a miss (what went wrong, the
  shortest route); 4. check progress; 5. set preferences (theme, font, keys, bindings).
- **Content inventory:** intent sentence; simulated terminal (cell grid: panes, borders, tmux
  status line); goal picture; keystroke strip (keycaps in tmux notation); session progress;
  result (grade word + icon, time vs target, "Shortest:" hint); due and new counts; streak
  strip; category names (Basics, Copy mode, Sessions, Layout); teach text; preferences.
- **Feel (Samuel, 2026-11-25):** fun, feels like the terminal, 80s / early-90s nostalgia.
- **Fixed by Samuel:**
  - Default font **Monaspace Argon**. All five Monaspace faces ship (Neon, Argon, Xenon, Radon,
    Krypton; v1.400, OFL). The user can choose another font.
  - **Themes** set the syntax/terminal colors and can be changed. Package the
    [Tomorrow themes](https://github.com/chriskempson/tomorrow-theme) (MIT); default
    **Tomorrow Night Bright**. Include other popular themes (Solarized etc.).
- **Constraints:** egui 0.37 (custom painting, no native menus; text shaping via harfrust and
  variable-font axes via `FontTweak`, so Monaspace weights and texture healing should work;
  check in the first snapshot test). WCAG 2.2 AA in every shipped theme. Reduced motion. No
  network. Every key except the pause chord goes to the simulator during practice.
- **Success:** a beginner knows what to do on first launch; practice feels like a terminal
  without feeling like homework; every theme and font combination stays readable.

### Assumptions I made

- "Syntax coloring" means the colors of the simulated terminal (prompt, paths, status line,
  copy-mode selection, pane borders). The app has no source code to highlight. One theme
  colors both the terminal and the app chrome.
- Light themes matter too (the spec has light / dark / follow system), so the theme setting has
  two slots: dark theme (default Tomorrow Night Bright) and light theme (default Tomorrow).
- Custom fonts come from a `.ttf`/`.otf` file the user picks, copied into the app-data folder
  (the only place Keyfluent writes). Listing installed system fonts needs an extra crate;
  left as an open question.
- No mood board: the brief already pins the feel.

## 2. Measured theme facts (contrast.py, WCAG 2.x, text = 4.5:1)

| Theme | As shipped | Fix |
|---|---|---|
| Tomorrow Night Bright | All text pairs pass on background (fg 17.5, comment 7.2, blue 8.3, red 5.0). Red on the current-line color is 3.5 | Use red only on `background` |
| Tomorrow Night | Passes except red 4.46 on bg | Red nudged lighter |
| Tomorrow Night Eighties | Passes on bg; comment, red and blue fail on current-line | Muted and accents used on `background` only |
| Tomorrow Night Blue | Comment 4.20 | Comment → `#778bbd` (4.53) |
| Tomorrow (light) | Comment 3.2, orange 2.5, yellow 1.9, green 3.9, aqua 3.4 | Muted → `#6c6d6a`; terminal accents darkened for text use |
| Solarized Dark / Light | Most accents 3–4.3; light fg 4.13; muted 2.2–2.8 | Nearest passing shade per hue (e.g. light muted `#93a1a1` → `#5e6b6c`) |
| Green / amber phosphor (new) | Text 14.4 / 10.8, muted 7.9 / 6.4, dim 3.9 / 3.4 | Dim level for borders only |

Rule proposed for every direction: **themes are tested content.** A test fails any shipped theme
whose role pairs fall under AA. A preference can show a theme's exact original colors instead.

## 3. Directions

Shared by all three: Monaspace (Argon default; separate terminal and interface font choices),
the theme engine above, keycaps for keys (tmux notation on hover), grades as word + icon.

### A · Mainframe
- **Concept:** the whole app is a text-mode program; every screen is a character grid with box
  frames, like a DOS-era tool or a BBS.
- **Mood:** immersive, crisp, nerdy. **Layout:** full-window double-line frame; menus as
  inverse-highlight lists; function-key bar (F1 Help, F2 Today, F3 Progress, F4 Preferences,
  outside practice only). **Density:** compact (one 13–14 px grid). **Type:** Argon only, one
  size; the intent line is double-width bold (a DEC double-size line). **Color:** theme as-is,
  frames in the comment color, inverse selection, one yellow marker. **Shape/motion:** 0 radius;
  blinking cursor only.
- **Signature move:** no visible GUI widgets: the app looks like it runs inside the terminal.
- **Best for:** terminal lovers; maximum nostalgia.
- **Risk:** hardest for a beginner who has never used a terminal; every control is a custom
  egui widget (more work, accessibility labels by hand); Preferences feels cramped.

### B · Workstation (safe choice)
- **Concept:** a normal desktop app (sidebar, panels, buttons) dressed like an early-90s Unix
  workstation, with the terminal as the hero.
- **Mood:** calm, capable, tactile. **Layout:** left rail (Today, Progress, Preferences),
  hidden during practice; content; goal picture in a side panel. **Density:** comfortable
  (base 15 px; 8/12/16 spacing). **Type:** Argon for UI and terminal, Xenon (slab) for headings
  and the intent; ratio 1.25. **Color:** theme surfaces + one accent (theme blue) for primary and
  focus; status colors only for grades. **Shape/motion:** 3 px radius, 1 px borders, no shadows;
  motion for feedback only.
- **Signature move:** chunky raised keycaps, like an old mechanical keyboard, wherever a key is shown.
- **Best for:** beginners; Preferences and Progress; the easiest egui build.
- **Risk:** the least "fun"; the retro feel lives in details, not structure.

### C · Boot screen
- **Concept:** early-90s home computer and arcade energy: big display type, a rainbow stripe, a
  score line after every card.
- **Mood:** playful, bold, warm. **Layout:** centered single column; Today is a title screen
  ("Press Enter to start"); Progress and Preferences are full screens from it. **Density:**
  spacious (base 16 px). **Type:** Krypton wide/heavy for display, Argon for body and terminal,
  Radon (handwriting) for teach notes. **Color:** all six theme accents; each category owns one
  (Basics blue, Copy mode green, Sessions purple, Layout orange). **Shape/motion:** square,
  dashed rules; score count-up (off with reduced motion).
- **Signature move:** the arcade score line (`GOOD ✓ 1.6 s / target 2.0 s`) and the six-color stripe.
- **Best for:** making daily practice feel like play.
- **Risk:** edges toward "gamified" (a spec anti-goal); big type crowds 1000 × 700; heavy color
  makes weaker themes garish and multiplies contrast checks.

**Why they differ:** layout (full-frame TUI / rail + panels / centered title screen), density
(compact / comfortable / spacious), type voice (one face / Argon + Xenon / Krypton + Radon mix),
color strategy (theme-literal / neutral + one accent / multi-accent), signature move.

## 4. Recommendation

**B as the structure, with two borrowings:** A's function-key-style bar and box-drawn panel
titles (the terminal feel, cheaply), and C's Monaspace mixing (Radon for teach notes, Krypton for
the result word). Reason: B serves beginners and the Preferences and Progress screens best, is
the cheapest and most accessible egui build, and these borrowings bring in the fun without
A's custom-widget cost or C's gamification risk.

## 5. Open questions

1. CRT effects (scanlines over the terminal): offer as a preference, off by default?
2. Custom fonts: file picker only, or also list installed system fonts (extra crate)?
3. Theme bundle beyond Tomorrow ×5, Solarized ×2 and phosphor ×2: Gruvbox, Dracula, Nord,
   Catppuccin (all MIT). All, some, or none?
4. Should the theme also color the app chrome, or only the terminal (with a fixed neutral app)?
   Assumed: both.

---

## 6. Round 1 decision and feedback (Samuel, 2026-11-25)

- **Direction C (Boot screen)** chosen: home computer and arcade energy, with the Monaspace mix.
  **Display text in all-caps Krypton** (sentence-case Krypton didn't look right). Card text stays
  sentence case in the data; the caps are a display style.
- **Gamification correction:** fun and game-like feedback are wanted ("dopamine hits" that help
  learning), but nothing that looks like it is *trying* to be a game. Spec §2 updated (Goal 9;
  anti-goal reworded).
- **From A:** function-key bar, box-titled panels (terminal, goal) in Practice, progress dots.
- **Answers:** no scanline effect. Installed system fonts allowed (e.g. via `fontdb`, MIT/Apache,
  local only), plus a font file; also bundle **Inconsolata, Source Code Pro, Fira Code** (OFL).
  Bundle **Gruvbox, Dracula, Nord, Catppuccin** too. The theme colors the **whole app**.
- **Asides recorded:** an About screen with linked credits for themes and fonts (spec §6.6); a
  docs website with a Themes page explaining the WCAG AA changes and the original-colors option
  (PLAN.md row "Website", dashboard task `kf-site`).

## 7. Round 2: four takes on C

Sheet: [`directions-v2.html`](directions-v2.html) (screenshot `directions-v2-shots/light-1440.png`).
Shared: all-caps Krypton display, Argon body/terminal, Radon notes; F-key bar, box-titled panels,
progress dots; score line, **New best**, combo (ends silently), session summary, "days this
month" (no streak to lose), scenarios as bonus rounds.

1. **Arcade cabinet:** C tightened. HUD row (card, combo, your best, target, in red arcade
   labels); result flash as a score line; a "Session complete" results table with new bests.
   Risk: one more row of numbers in practice (HUD can be hidden).
2. **Home computer:** a thick border around the whole window in the category color; on success
   it flashes loading-stripes, on a miss it turns red. Today is a boot screen ("READY. / PRESS
   ENTER TO START"). Risk: intense flashing (kept under 3 per second, can be off); 44 px of width.
3. **Cartridge:** stars per card = the FSRS grade (Again 0 … Easy 3); Progress as a stage-select
   grid of card tiles colored by mastery band, scenarios as locked bonus tiles. Shown in Tomorrow
   Night Eighties. Risk: stars are the most overtly game-like element.
4. **Cracktro (out there):** Today is a demoscene intro: raster bars in the theme accents, a
   sine-wave logo, a starfield, and a scroller with tips and scene-style greetings to the theme
   and font authors. Practice stays calm; success sweeps a raster bar; session end scrolls past.
   Optional chiptune sounds (off by default). Risk: heavy motion (all still with reduced motion);
   the Today screen redraws every frame.

These parts combine freely: e.g. 1's HUD + 3's stars and stage select + 4's Today screen.

**Recommendation:** **1 as the base, plus 3's stage-select Progress page, plus 4's cracktro as the
Today screen** (still when reduced motion is on). Stars (3) and the window border (2) are optional
extras. Reason: 1 keeps practice readable; the stage select makes the spec's mastery heatmap
something you want to fill in; the cracktro gives the app its memorable opening without touching
practice.

**Measured in this round:** black text on every Tomorrow Night Bright accent passes (5.0–12.5);
muted gray on the F-key bar fails in Eighties (4.05), so F-key bar hints use normal text color.

## 8. Round 2 decision (Samuel, 2026-11-25)

Base = 1 Arcade cabinet with the title about 20% smaller and the HUD numbers hidden; stars from 3;
Progress = 3's stage select; Today = 4's cracktro title screen with the logo in **Xenon, title
case**; Session end = 4's starfield. Chiptune sounds yes, plus a **dial-up modem sound on launch**.
Recorded in [`DESIGN.md`](../../DESIGN.md); key screens in [`mockups.html`](mockups.html).
