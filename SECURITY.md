# Security policy

## Supported versions

Security fixes target the latest published release and the current `master` branch. Older releases may not receive backports; users should reproduce a suspected issue on the latest version when safe to do so.

## Reporting a vulnerability

Do not open a public issue for vulnerabilities that could expose credentials, execute unintended commands, escape filesystem boundaries, replace release artifacts, or otherwise harm users.

Use GitHub's **Report a vulnerability** option on the repository Security tab to create a private security advisory:

https://github.com/vorlie/ani-cli-rs/security/advisories/new

Include:

- The affected version, operating system, and architecture.
- Reproduction steps or a minimal proof of concept.
- The expected security boundary and practical impact.
- Any suggested mitigation, if known.

Remove real credentials, cookies, authorization headers, and active media URLs. Use synthetic values wherever possible. Please allow reasonable time to investigate and prepare a release before publishing details.

Provider outages, expired links, antivirus heuristics without an exploitable security impact, and ordinary playback failures should use the public issue templates instead.

