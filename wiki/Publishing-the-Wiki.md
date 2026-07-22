# Publishing the Wiki

The canonical editable wiki source is kept in the main repository's `wiki/` directory so changes can be reviewed and versioned with the code.

GitHub stores the rendered Wiki in a separate Git repository:

```text
https://github.com/vorlie/ani-cli-rs.wiki.git
```

## First-time setup

GitHub does not create the separate Wiki repository until the Wiki feature is enabled and its first page exists.

1. Open the repository **Settings → General → Features**.
2. Enable **Wikis**.
3. Open the **Wiki** tab.
4. Create an initial `Home` page with any temporary text.
5. Clone the Wiki repository.

```console
git clone https://github.com/vorlie/ani-cli-rs.wiki.git ani-cli-rs-wiki
```

## Publish the tracked source

From a checkout of the main repository, copy the Markdown files from `wiki/` into the root of the Wiki clone. Preserve filenames such as `Home.md` and `_Sidebar.md`.

PowerShell example:

```powershell
$source = Resolve-Path .\wiki
$wiki = Resolve-Path ..\ani-cli-rs-wiki
Copy-Item "$source\*.md" $wiki -Force
git -C $wiki add --all
git -C $wiki commit -m "docs: publish comprehensive project wiki"
git -C $wiki push origin master
```

Shell example:

```sh
cp wiki/*.md ../ani-cli-rs-wiki/
git -C ../ani-cli-rs-wiki add --all
git -C ../ani-cli-rs-wiki commit -m "docs: publish comprehensive project wiki"
git -C ../ani-cli-rs-wiki push origin master
```

Check the Wiki clone's current branch before pushing; GitHub Wiki repositories commonly use `master`, but use the branch reported by `git branch --show-current` rather than assuming.

## Editing policy

- Prefer editing the tracked `wiki/` source and republishing it.
- If a page is hotfixed through GitHub's web UI, copy that change back into `wiki/` immediately.
- Keep `_Sidebar.md` synchronized when adding, renaming, or removing pages.
- Use Wiki-compatible page links without `.md`, for example `[Downloads](Downloads)`.
- Never copy release secrets, private issue material, or signed provider URLs into the Wiki.

## Validation before publishing

Check that:

- every `_Sidebar` page exists;
- internal Markdown links resolve;
- examples match current `--help` output;
- release asset names and supported targets match packaging scripts;
- version-specific behavior states the first applicable version;
- the main repository remains the source of truth for security policy and scraper implementation details.
