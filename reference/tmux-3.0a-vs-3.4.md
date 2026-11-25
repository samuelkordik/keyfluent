# tmux 3.0a vs 3.4 key tables

Compares `reference/tmux-3.0a-list-keys.txt` with
`crates/kf-tmux/data/tmux-3.4-list-keys.txt` (stock `list-keys`, Ubuntu 24.04,
generated 2026-11-25). Keys are compared after unescaping (`\%` = `%`,
`"M-{"` = `M-{`); commands after collapsing whitespace. `-r` marks a
repeatable binding. Commands longer than 110 characters are cut with `…`.
Rows marked "(mouse)" are mouse bindings, which Keyfluent's parser skips.

## Basics bindings

These prefix-table bindings are **identical** in 3.0a and 3.4:
`%` `"` `Left` `Right` `Up` `Down` `o` `;` `z` `x` `c` `n` `p` `0`–`9` `l` `&`
(including the `-r` flag on the arrow keys).

One differs in how it is written, not in what it does:

- `,`: 3.0a `command-prompt -I "#W" "rename-window -- '%%'"`, 3.4
  `command-prompt -I "#W" { rename-window "%%" }`. Both open a prompt
  pre-filled with the window name and rename the window to the answer. 3.4
  writes the template as a `{ … }` block and drops the `--`. For the learner the
  keys, prompt and result are the same.

Most other changes below follow the same pattern: 3.4 rewrites quoted
`command-prompt` templates as `{ … }` blocks, adds `-T <prompt type>` to some
prompts, and extends the menus.

## `copy-mode`

68 bindings in 3.0a, 72 in 3.4; 46 identical.

### Added (4)

| Key | Command |
|---|---|
| `P` | `send-keys -X toggle-position` |
| `X` | `send-keys -X set-mark` |
| `r` | `send-keys -X refresh-from-pane` |
| `M-x` | `send-keys -X jump-to-mark` |

### Removed (0)

None.

### Changed (22)

| Key | 3.0a | 3.4 |
|---|---|---|
| `C-k` | `send-keys -X copy-end-of-line` | `send-keys -X copy-pipe-end-of-line-and-cancel` |
| `C-r` | `command-prompt -i -I "#{pane_search_string}" -p "(search up)" "send -X search-backward-incremental \"%%%\""` | `command-prompt -i -I "#{pane_search_string}" -T search -p "(search up)" { send-keys -X search-backward-increme …` |
| `C-s` | `command-prompt -i -I "#{pane_search_string}" -p "(search down)" "send -X search-forward-incremental \"%%%\""` | `command-prompt -i -I "#{pane_search_string}" -T search -p "(search down)" { send-keys -X search-forward-increm …` |
| `C-w` | `send-keys -X copy-selection-and-cancel` | `send-keys -X copy-pipe-and-cancel` |
| `F` | `command-prompt -1 -p "(jump backward)" "send -X jump-backward \"%%%\""` | `command-prompt -1 -p "(jump backward)" { send-keys -X jump-backward "%%" }` |
| `T` | `command-prompt -1 -p "(jump to backward)" "send -X jump-to-backward \"%%%\""` | `command-prompt -1 -p "(jump to backward)" { send-keys -X jump-to-backward "%%" }` |
| `f` | `command-prompt -1 -p "(jump forward)" "send -X jump-forward \"%%%\""` | `command-prompt -1 -p "(jump forward)" { send-keys -X jump-forward "%%" }` |
| `g` | `command-prompt -p "(goto line)" "send -X goto-line \"%%%\""` | `command-prompt -p "(goto line)" { send-keys -X goto-line "%%" }` |
| `t` | `command-prompt -1 -p "(jump to forward)" "send -X jump-to-forward \"%%%\""` | `command-prompt -1 -p "(jump to forward)" { send-keys -X jump-to-forward "%%" }` |
| `MouseDragEnd1Pane` (mouse) | `send-keys -X copy-selection-and-cancel` | `send-keys -X copy-pipe-and-cancel` |
| `DoubleClick1Pane` (mouse) | `select-pane \; send-keys -X select-word` | `select-pane \; send-keys -X select-word \; run-shell -d 0.3 \; send-keys -X copy-pipe-and-cancel` |
| `TripleClick1Pane` (mouse) | `select-pane \; send-keys -X select-line` | `select-pane \; send-keys -X select-line \; run-shell -d 0.3 \; send-keys -X copy-pipe-and-cancel` |
| `M-1` | `command-prompt -N -I 1 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 1 -p (repeat) { send-keys -N "%%" }` |
| `M-2` | `command-prompt -N -I 2 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 2 -p (repeat) { send-keys -N "%%" }` |
| `M-3` | `command-prompt -N -I 3 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 3 -p (repeat) { send-keys -N "%%" }` |
| `M-4` | `command-prompt -N -I 4 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 4 -p (repeat) { send-keys -N "%%" }` |
| `M-5` | `command-prompt -N -I 5 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 5 -p (repeat) { send-keys -N "%%" }` |
| `M-6` | `command-prompt -N -I 6 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 6 -p (repeat) { send-keys -N "%%" }` |
| `M-7` | `command-prompt -N -I 7 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 7 -p (repeat) { send-keys -N "%%" }` |
| `M-8` | `command-prompt -N -I 8 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 8 -p (repeat) { send-keys -N "%%" }` |
| `M-9` | `command-prompt -N -I 9 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 9 -p (repeat) { send-keys -N "%%" }` |
| `M-w` | `send-keys -X copy-selection-and-cancel` | `send-keys -X copy-pipe-and-cancel` |

## `copy-mode-vi`

78 bindings in 3.0a, 87 in 3.4; 56 identical.

### Added (9)

| Key | Command |
|---|---|
| `#` | `send-keys -FX search-backward "#{copy_cursor_word}"` |
| `*` | `send-keys -FX search-forward "#{copy_cursor_word}"` |
| `P` | `send-keys -X toggle-position` |
| `X` | `send-keys -X set-mark` |
| `r` | `send-keys -X refresh-from-pane` |
| `z` | `send-keys -X scroll-middle` |
| `Home` | `send-keys -X start-of-line` |
| `End` | `send-keys -X end-of-line` |
| `M-x` | `send-keys -X jump-to-mark` |

### Removed (0)

None.

### Changed (22)

| Key | 3.0a | 3.4 |
|---|---|---|
| `C-j` | `send-keys -X copy-selection-and-cancel` | `send-keys -X copy-pipe-and-cancel` |
| `Enter` | `send-keys -X copy-selection-and-cancel` | `send-keys -X copy-pipe-and-cancel` |
| `/` | `command-prompt -p "(search down)" "send -X search-forward \"%%%\""` | `command-prompt -T search -p "(search down)" { send-keys -X search-forward "%%" }` |
| `1` | `command-prompt -N -I 1 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 1 -p (repeat) { send-keys -N "%%" }` |
| `2` | `command-prompt -N -I 2 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 2 -p (repeat) { send-keys -N "%%" }` |
| `3` | `command-prompt -N -I 3 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 3 -p (repeat) { send-keys -N "%%" }` |
| `4` | `command-prompt -N -I 4 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 4 -p (repeat) { send-keys -N "%%" }` |
| `5` | `command-prompt -N -I 5 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 5 -p (repeat) { send-keys -N "%%" }` |
| `6` | `command-prompt -N -I 6 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 6 -p (repeat) { send-keys -N "%%" }` |
| `7` | `command-prompt -N -I 7 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 7 -p (repeat) { send-keys -N "%%" }` |
| `8` | `command-prompt -N -I 8 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 8 -p (repeat) { send-keys -N "%%" }` |
| `9` | `command-prompt -N -I 9 -p (repeat) "send -N \"%%%\""` | `command-prompt -N -I 9 -p (repeat) { send-keys -N "%%" }` |
| `:` | `command-prompt -p "(goto line)" "send -X goto-line \"%%%\""` | `command-prompt -p "(goto line)" { send-keys -X goto-line "%%" }` |
| `?` | `command-prompt -p "(search up)" "send -X search-backward \"%%%\""` | `command-prompt -T search -p "(search up)" { send-keys -X search-backward "%%" }` |
| `D` | `send-keys -X copy-end-of-line` | `send-keys -X copy-pipe-end-of-line-and-cancel` |
| `F` | `command-prompt -1 -p "(jump backward)" "send -X jump-backward \"%%%\""` | `command-prompt -1 -p "(jump backward)" { send-keys -X jump-backward "%%" }` |
| `T` | `command-prompt -1 -p "(jump to backward)" "send -X jump-to-backward \"%%%\""` | `command-prompt -1 -p "(jump to backward)" { send-keys -X jump-to-backward "%%" }` |
| `f` | `command-prompt -1 -p "(jump forward)" "send -X jump-forward \"%%%\""` | `command-prompt -1 -p "(jump forward)" { send-keys -X jump-forward "%%" }` |
| `t` | `command-prompt -1 -p "(jump to forward)" "send -X jump-to-forward \"%%%\""` | `command-prompt -1 -p "(jump to forward)" { send-keys -X jump-to-forward "%%" }` |
| `MouseDragEnd1Pane` (mouse) | `send-keys -X copy-selection-and-cancel` | `send-keys -X copy-pipe-and-cancel` |
| `DoubleClick1Pane` (mouse) | `select-pane \; send-keys -X select-word` | `select-pane \; send-keys -X select-word \; run-shell -d 0.3 \; send-keys -X copy-pipe-and-cancel` |
| `TripleClick1Pane` (mouse) | `select-pane \; send-keys -X select-line` | `select-pane \; send-keys -X select-line \; run-shell -d 0.3 \; send-keys -X copy-pipe-and-cancel` |

## `prefix`

83 bindings in 3.0a, 85 in 3.4; 74 identical.

### Added (2)

| Key | Command |
|---|---|
| `/` | `command-prompt -k -p key { list-keys -1N "%%" }` |
| `C` | `customize-mode -Z` |

### Removed (0)

None.

### Changed (9)

| Key | 3.0a | 3.4 |
|---|---|---|
| `$` | `command-prompt -I "#S" "rename-session -- '%%'"` | `command-prompt -I "#S" { rename-session "%%" }` |
| `'` | `command-prompt -p index "select-window -t ':%%'"` | `command-prompt -T window-target -p index { select-window -t ":%%" }` |
| `,` | `command-prompt -I "#W" "rename-window -- '%%'"` | `command-prompt -I "#W" { rename-window "%%" }` |
| `.` | `command-prompt "move-window -t '%%'"` | `command-prompt -T target { move-window -t "%%" }` |
| `<` | `display-menu -T "#[align=centre]#{window_index}:#{window_name}" -x W -y S "Swap Left" l "swap-window -t:-1" "S …` | `display-menu -T "#[align=centre]#{window_index}:#{window_name}" -x W -y W "#{?#{>:#{session_windows},1},,-}Swa …` |
| `>` | `display-menu -T "#[align=centre]#{pane_index} (#{pane_id})" -x P -y P "#{?mouse_word,Search For #[underscore]# …` | `display-menu -T "#[align=centre]#{pane_index} (#{pane_id})" -x P -y P "#{?#{m/r:(copy\|view)-mode,#{pane_mode}} …` |
| `?` | `list-keys` | `list-keys -N` |
| `]` | `paste-buffer` | `paste-buffer -p` |
| `f` | `command-prompt "find-window -Z -- '%%'"` | `command-prompt { find-window -Z "%%" }` |

## `root`

12 bindings in 3.0a, 16 in 3.4; 5 identical.

### Added (5)

| Key | Command |
|---|---|
| `MouseDown2Pane` (mouse) | `select-pane -t = \; if-shell -F "#{\|\|:#{pane_in_mode},#{mouse_any_flag}}" { send-keys -M } { paste-buffer -p }` |
| `DoubleClick1Pane` (mouse) | `select-pane -t = \; if-shell -F "#{\|\|:#{pane_in_mode},#{mouse_any_flag}}" { send-keys -M } { copy-mode -H ; se …` |
| `TripleClick1Pane` (mouse) | `select-pane -t = \; if-shell -F "#{\|\|:#{pane_in_mode},#{mouse_any_flag}}" { send-keys -M } { copy-mode -H ; se …` |
| `M-MouseDown3Status` (mouse) | `display-menu -T "#[align=centre]#{window_index}:#{window_name}" -t = -x W -y W "#{?#{>:#{session_windows},1},, …` |
| `M-MouseDown3StatusLeft` (mouse) | `display-menu -T "#[align=centre]#{session_name}" -t = -x M -y W Next n { switch-client -n } Previous p { switc …` |

### Removed (1)

| Key | Command |
|---|---|
| `MouseDown3StatusRight` (mouse) | `display-menu -T "#[align=centre]#{client_name}" -t = -x M -y S Detach d detach-client "Detach & Kill" X "detac …` |

### Changed (6)

| Key | 3.0a | 3.4 |
|---|---|---|
| `MouseDown3Pane` (mouse) | `if-shell -F -t = "#{\|\|:#{mouse_any_flag},#{pane_in_mode}}" "select-pane -t=; send-keys -M" "display-menu -t= - …` | `if-shell -F -t = "#{\|\|:#{mouse_any_flag},#{&&:#{pane_in_mode},#{?#{m/r:(copy\|view)-mode,#{pane_mode}},0,1}}}"  …` |
| `MouseDown3Status` (mouse) | `display-menu -T "#[align=centre]#{window_index}:#{window_name}" -t = -x W -y S "Swap Left" l "swap-window -t:- …` | `display-menu -T "#[align=centre]#{window_index}:#{window_name}" -t = -x W -y W "#{?#{>:#{session_windows},1},, …` |
| `MouseDown3StatusLeft` (mouse) | `display-menu -T "#[align=centre]#{session_name}" -t = -x M -y S Next n "switch-client -n" Previous p "switch-c …` | `display-menu -T "#[align=centre]#{session_name}" -t = -x M -y W Next n { switch-client -n } Previous p { switc …` |
| `MouseDrag1Pane` (mouse) | `if-shell -F -t = "#{mouse_any_flag}" "if -Ft= \"#{pane_in_mode}\" \"copy-mode -M\" \"send-keys -M\"" "copy-mod …` | `if-shell -F "#{\|\|:#{pane_in_mode},#{mouse_any_flag}}" { send-keys -M } { copy-mode -M }` |
| `WheelUpPane` (mouse) | `if-shell -F -t = "#{mouse_any_flag}" "send-keys -M" "if -Ft= \"#{pane_in_mode}\" \"send-keys -M\" \"copy-mode  …` | `if-shell -F "#{\|\|:#{pane_in_mode},#{mouse_any_flag}}" { send-keys -M } { copy-mode -e }` |
| `M-MouseDown3Pane` (mouse) | `display-menu -T "#[align=centre]#{pane_index} (#{pane_id})" -t = -x M -y M "#{?mouse_word,Search For #[undersc …` | `display-menu -T "#[align=centre]#{pane_index} (#{pane_id})" -t = -x M -y M "#{?#{m/r:(copy\|view)-mode,#{pane_m …` |
