# Migrating from the Wiki

> **Deprecation notice — effective immediately**
>
> The GitHub Wiki for `vorlie/ani-cli-rs` is **no longer the canonical
> documentation source**. As of the next minor release, this repository's
> `wiki/` directory is in a one-release **deprecation window** and will be
> removed in a follow-up commit.
>
> **All new and updated documentation must be edited under
> `website/docs/`.** The MkDocs Material site published from
> `website/` is the single source of truth.

## Where to edit now

| Topic | New path |
|---|---|
| Home | `website/docs/index.md` |
| Installation | `website/docs/guides/installation.md` |
| Getting Started | `website/docs/guides/getting-started.md` |
| Playback and Players | `website/docs/guides/playback-and-players.md` |
| Downloads | `website/docs/guides/downloads.md` |
| Configuration and History | `website/docs/guides/configuration.md` |
| CLI Reference | `website/docs/reference/cli.md` |
| FAQ | `website/docs/faq.md` |
| Support / Troubleshooting | `website/docs/support/index.md`, `website/docs/support/troubleshooting.md` |
| Provider Architecture | `website/docs/development/architecture.md` |
| Anikoto API / KotoCDN | `website/docs/development/anikoto-kotocdn.md` |
| Anikoto.cz | `website/docs/development/anikoto-cz.md` |
| Security and Privacy | `website/docs/development/security.md` |
| Building and Releasing | `website/docs/development/building.md` |
| Contributing | `website/docs/development/contributing.md` |

The `mkdocs.yml` `nav:` block controls navigation, sidebar, and search
indexing. Add new pages there too.

## What happens to this directory

- `wiki/` is **not ignored** for one release cycle. It is kept only so
  external links into the GitHub Wiki can still be hand-fixed by
  redirecting through the site.
- After the next minor release, `wiki/` will be deleted in a single
  focused commit (`chore(docs): remove deprecated wiki/ directory`).
- The GitHub Wiki feature should be disabled in repository settings
  shortly after that delete lands.

## During the deprecation window

If you must edit a page here (for example, a critical fix that cannot
wait for the website rebuild), use the migration script to copy the
change into `website/docs/`:

```sh
./scripts/sync-wiki-to-website.sh
```

```powershell
./scripts/sync-wiki-to-website.ps1
```

The script is **one-way** (`wiki/` → `website/docs/`). It rewrites
wiki-style `[Page](PageName)` links into the corresponding MkDocs
paths. It does not edit anything under `wiki/` itself.

## Validation before opening a PR

```sh
.venv/Scripts/python.exe -m mkdocs build --strict
```

The site must build with no unrecognized-link warnings before any
documentation change is merged.

## Why

- The MkDocs Material site gives a proper navigation tree, full-text
  search, dark/light themes, and a single place to review
  documentation diffs alongside code.
- The GitHub Wiki split repository forced copy-paste publishing,
  silently drifted from the tracked source, and had no preview or link
  validation.
- A single tracked source removes the dual-edit problem and keeps
  releases reproducible.

## Questions

Open an issue or discussion if the migration affects a workflow you
maintain. The deprecation window exists precisely to surface those
cases.
