# ani-cli-rs 0.10.2

This patch release fixes the search flow and brings the default provider back in line with the live Anikoto.cz catalog used by real results.

## Highlights

- Restored the Anikoto.cz filter page as the canonical search source
- Fixed stale API assumptions that dropped real matches or returned incomplete results
- Improved result parsing and deduplication for the live filter page
- Kept the CLI help output readable and styled for interactive terminals without breaking non-TTY/script output
- Preserved log file behavior and environment overrides for troubleshooting

## Fixes

- Fixed provider default drift and made the current catalog behavior consistent again
- Corrected parsing of `#list-items` results to ignore unrelated markup outside the real result list
- Deduplicated duplicate show entries and normalized slug handling for watch URLs
- Improved search sort handling and generated filter URLs for supported sort variants
- Updated CLI styling to use auto color detection so scripts and tests keep plain output while terminals still get the warm rust/orange theme

## Notes

- The live Anikoto.cz search flow is now the default path for real catalog data
- This is a patch release; no breaking CLI or API changes are expected for existing users

## Upgrade

Existing installations can update with:

```console
ani-cli-rs update
```

**Full changelog:** [0.10.1...0.10.2](https://github.com/vorlie/ani-cli-rs/compare/0.10.1...0.10.2)