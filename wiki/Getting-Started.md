# Getting Started

## Play an anime interactively

```console
ani-cli-rs "frieren"
```

The normal flow is:

1. search the selected Anikoto catalog;
2. choose the correct anime entry or season;
3. choose an episode;
4. resolve supported sources and requested quality;
5. launch the platform player: mpv, IINA, or an Android player from Termux;
6. use the post-launch menu to choose next, replay, previous, another episode, or another quality.

If the query is omitted, ani-cli-rs prompts for it:

```console
ani-cli-rs
```

## Subbed, dubbed, and adult results

```console
ani-cli-rs --dub "cowboy bebop"
ani-cli-rs --allow-adult "search query"
```

`--dub` changes the translation catalog. `--allow-adult` permits adult-marked search results; it does not disable network or router filtering.

## Quality

```console
ani-cli-rs -q best "title"
ani-cli-rs -q worst "title"
ani-cli-rs -q 720p "title"
```

An explicit resolution falls back to the best resolved stream when that exact resolution is absent.

## Choose episodes without a prompt

```console
ani-cli-rs -e 1 "title"
ani-cli-rs -e 2-5 "title"
ani-cli-rs -e "1 3 5.5" "title"
```

`-r/--range` is an alias of `-e/--episode`. Fractional episode labels are preserved when the selected catalog publishes them.

## Multiple interactive episodes

Choose **Select multiple episodes** in the episode picker, or open it directly:

```console
ani-cli-rs --multi-selection "title"
```

Use Space to toggle entries and Enter to confirm.

## Download by title

```console
ani-cli-rs --download "anime title"
```

Select the anime/season and episodes. Version 0.6.0 and newer preflight every selected episode before starting the first transfer. See [Downloads](Downloads).

## Continue from history

```console
ani-cli-rs --continue
```

Select a history entry and ani-cli-rs chooses its next published episode. History is compatible with Bash ani-cli's tab-separated `ani-hsts` format.

## Keyboard navigation

| Key | Fuzzy anime/episode menus | Ordinary action menus |
|---|---|---|
| Arrow keys | Move | Move |
| Tab / Shift+Tab | Down / up | Down / up |
| Type | Filter results | — |
| Space | Toggle in multi-select | Select |
| Enter | Select/confirm | Select |
| Escape | Go back | Cancel/quit |
| `j` / `k` | Typed as filter text | Down / up |
| `h` / `l` | Typed as filter text | Previous / next page |
| `q` | Typed as filter text | Cancel/quit |

The visible Back row has the same effect as Escape. In fuzzy search, `q` remains available for titles containing that character.

## Next episode schedule

```console
ani-cli-rs --nextep-countdown "anime title"
```

This displays the next scheduled Japanese/raw and subtitled release timestamps from AnimeSchedule, then exits.
