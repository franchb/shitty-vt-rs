# shitty-vt-rs

Rust bindings to [shitty](https://github.com/pg83/shitty)'s embeddable VT core.

- `shitty-vt-sys` — raw FFI declarations, transcribed by hand from
  `lib/embed/shitty_vt.h`. No bindgen, so no libclang on consumers.
- `shitty-vt` — a safe wrapper: feed bytes, read a grid.

Neither crate depends on any terminal application. The core owns no pty and
spawns no child; replies it generates queue until the embedder drains them.

## Building

Build the facade in a shitty checkout first:

```sh
cd /path/to/shitty
./build a     # static  -> .build/libshitty_vt.a
./build so    # shared  -> .build/libshitty_vt.so
```

Then point the crates at it:

```sh
# shared
SHITTY_VT_LIB_DIR=/path/to/shitty/.build \
LD_LIBRARY_PATH=/path/to/shitty/.build \
cargo test

# static (self-contained at run time)
SHITTY_VT_LIB_DIR=/path/to/shitty/.build \
SHITTY_VT_STATIC=1 \
SHITTY_VT_LINK_LIBS=xxhash,atomic,uring \
cargo test
```

`SHITTY_VT_LINK_LIBS` exists because the static archive cannot bundle the
system libraries libstd probed for, and which those are varies by host. On a
box with `rapidhash.h` there is no `xxhash` to name; on one without `liburing`
there is no `uring`. Check `readelf -d` on the shared library, or the build's
own link line, to see what your host chose.

## Status

Working: feed, resize, per-cell reads with grapheme clusters and resolved
colours, cursor, mode flags, reply draining, scrollback view movement,
row-addressed history reads, memory accounting, a changeable history cap, and
the title, bell, damage, open-uri, clipboard and resize-request callbacks.
Twenty-two behaviour tests
cover these, in the same cell-dump format the Luvus conformance tests use so
the two engines can be diffed directly.

### Mapping onto Luvus's `VtEngine`

Written down because the gaps are the interesting part, not the matches.

| `VtEngine` | facade |
|---|---|
| `advance` | `shitty_vt_feed` |
| `resize` | `shitty_vt_resize` |
| `for_each_cell` | `shitty_vt_each_cell` — same convention, continuations skipped |
| `cursor` | `shitty_vt_cursor_state` |
| `alt_screen`, `bracketed_paste`, `application_cursor`, `mouse_report`, `mouse_drag`, `mouse_motion`, `sgr_mouse` | `shitty_vt_modes` bits |
| terminal replies | `shitty_vt_take_replies` |
| `title` | `title_changed` callback |
| `visible_rows`, `detection_text`, `codex_composer_region` | derivable from cells |
| `output_generation`, `finish_output_batch` | no equivalent needed; track locally |
| `snapshot_ansi` | not exposed; synthesizable from cells and attributes |
| `scroll`, `scroll_to`, `scroll_to_top`, `scroll_to_bottom`, `scroll_offset`, `history_len` | `shitty_vt_scroll`, `shitty_vt_scroll_to`, `shitty_vt_scroll_offset`, `shitty_vt_history_rows` — needs pg83/shitty#99 |
| `retained_row_text`, `for_each_retained_row`, `retained_row_count` | `shitty_vt_row_cells`, `shitty_vt_total_rows` — needs pg83/shitty#100 |
| `history_metrics` | `shitty_vt_memory_usage` — cells only, so report it as an estimate rather than exact |
| `set_history_budget` | `shitty_vt_set_save_lines` — a row cap, so a byte budget has to be divided by the row cost the same call reports |
| `alternate_scroll` | `shitty_vt_modes` bit 15 — needs pg83/shitty#101 |

The visible grid and the scrollback are both covered once pg83/shitty#99 and
pg83/shitty#100 land; until they do, the scrolling, row-reading and budgeting
calls need a shitty checkout carrying them, and `Modes::alternate_scroll` needs
pg83/shitty#101. With those, every method on Luvus's `VtEngine` has something
behind it — though `history_metrics` and `set_history_budget` are honest partial
matches rather than exact ones, as the table notes.

### Known quirks

- The facade publishes an empty title at construction, so `title()` reads
  `Some("")` before the application sets anything. Treat emptiness rather than
  `None` as "no title yet". Reported as pg83/shitty#98.
- `Cursor::row` is a row of the current view, so while scrolled into the
  scrollback it can be at or past the last row, meaning the cursor is off
  screen. Do not index a grid with it unchecked.
- Emoji width differs from `alacritty_terminal`: a ZWJ sequence or an
  emoji-modifier sequence is one width-2 cell here and two wide cells there,
  and `U+2764 U+FE0F` is wide here and narrow there. UTS #51 favours this
  reading. The tests pin it in both directions.

### Threading

`Terminal` is `Send` but not `Sync`: the facade exposes one opaque instance
with no shared global state, and every entry point takes that instance. Moving
one between threads is fine; touching one from two at once is not. This is
inferred from the interface — upstream documents no threading contract — so
treat it as an assumption that wants confirming before anything leans on it.

## Licence

GPL-3.0-or-later, following the licence of the shitty VT core it links.
