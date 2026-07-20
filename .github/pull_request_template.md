## Summary

Describe the problem and the resulting behavior. Keep unrelated changes in separate pull requests.

## Related issue

Closes #

## Testing

List the commands and platforms used to verify the change.

```console
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```

## Checklist

- [ ] The change is focused and its commit history is understandable.
- [ ] User-facing flags, environment variables, or behavior are documented.
- [ ] New scraper behavior has deterministic fixtures or mock-server coverage.
- [ ] No credentials, cookies, complete signed media URLs, personal paths, or generated debug dumps are committed.
- [ ] Existing Bash ani-cli compatibility is preserved or an intentional difference is explained.
- [ ] I have tested relevant player, downloader, installer, or platform-specific behavior where practical.

