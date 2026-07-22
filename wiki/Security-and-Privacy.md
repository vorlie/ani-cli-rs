# Security and Privacy

## Release trust

Official Windows binaries are currently unsigned. Microsoft SmartScreen and heuristic antivirus products may warn about uncommon unsigned executables. A warning should be investigated, not automatically ignored.

Older VirusTotal behavior reports have described parts of ani-cli-rs as obfuscated. Expected implementation behaviors that can contribute to heuristic classifications include:

- AES-GCM, AES-CTR, P-256, XOR, SHA-256, and Base64 operations used by AllAnime and providers;
- downloading and scanning Mkissa frontend JavaScript for rotating crypto material;
- resolving temporary media URLs;
- launching mpv, VLC, Syncplay, aria2c, yt-dlp, or FFmpeg;
- an installer modifying the current user's `PATH`.

The application does not require administrator privileges.

## Verify a release archive

Every release package must have a matching `.sha256` asset. The install scripts verify it automatically.

PowerShell using built-in .NET hashing:

```powershell
$path = ".\ani-cli-rs-VERSION-x86_64-pc-windows-msvc.zip"
$stream = [System.IO.File]::OpenRead($path)
try {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($stream))).Replace("-", "") }
    finally { $sha.Dispose() }
} finally { $stream.Dispose() }
```

Linux:

```sh
sha256sum ani-cli-rs-VERSION-x86_64-unknown-linux-musl.tar.gz
cat ani-cli-rs-VERSION-x86_64-unknown-linux-musl.tar.gz.sha256
```

A matching hash proves the file is byte-for-byte identical to the published release asset. It does not independently prove the publisher or source is trustworthy.

## Build from tagged source

Users who prefer not to trust a prebuilt binary can inspect a release tag and run:

```console
cargo build --release --locked
```

Reproducible byte-for-byte output is not promised across compiler versions, linkers, operating systems, or build paths.

## Network behavior

Depending on the command, ani-cli-rs contacts:

- Mkissa/AllAnime bootstrap and GraphQL endpoints;
- third-party media/provider hosts returned for the selected episode;
- upstream Bash ani-cli when explicitly refreshing the cipher map;
- AnimeSchedule for `--nextep-countdown`;
- GitHub Releases for update checks and installation.

Third-party services see ordinary network metadata such as public IP address, TLS/client characteristics, user agent, and requested resource. Their privacy policies are outside this project's control.

## Logs and issue reports

Routine scraper logs redact complete signed URLs, but users should still inspect logs. Never publish:

- cookies or authorization headers;
- complete temporary media query strings;
- active provider tokens;
- decrypted raw episode payloads;
- personal home/build paths;
- private network or account identifiers.

## Vulnerability reporting

Do not open a public issue for command execution, credential exposure, path escapes, release replacement, or similar security impact. Use GitHub's private advisory form:

<https://github.com/vorlie/ani-cli-rs/security/advisories/new>

Provider outages, expired URLs, ordinary antivirus heuristics, and playback failures belong in public bug reports.

## External plugins

The plugin system is a roadmap, not a shipped feature. The proposed design uses external executables rather than Rust `.dll`/`.so` loading. Such plugins would be trusted native code running as the current user; no plugin should be installed or executed merely because it appears in the current directory.
