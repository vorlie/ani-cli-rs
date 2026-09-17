> This file is the canonical source of agent instructions for this repository.
> `CLAUDE.md`, `GEMINI.md`, and `.cursor/rules/agents.mdc` are thin pointers to
> this file for tools that don't read `AGENTS.md` by default — edit this file,
> not those.

# AGENTS.md

Instructions for AI coding agents working on ani-cli-rs.

## Project overview

ani-cli-rs is a fast, cross-platform anime CLI written in Rust.

The project provides both:

* A command-line application
* The `ani_cli` Rust library

The project currently integrates multiple anime catalogs and external streaming providers. Provider behavior can change independently of this repository, so provider maintenance is an important part of development.

## Before making changes

Before modifying code:

1. Read this file completely.
2. Read the relevant project documentation.
3. Inspect the existing implementation before proposing a new abstraction.
4. Check the current Rust toolchain and dependency versions.
5. Search the repository for existing implementations of the functionality you need.
6. Determine whether the change affects the CLI, library API, provider implementations, or multiple layers.

Do not assume an API, provider behavior, function, module, or dependency exists without checking the repository.

## Project principles

Prefer:

* Small, focused changes
* Existing project abstractions
* Clear Rust code
* Strong typing
* Explicit error handling
* Reusable library functionality
* Minimal dependencies
* Cross-platform behavior

Avoid:

* Unnecessary rewrites
* Large unrelated refactors
* New abstractions without a concrete need
* Duplicating existing functionality
* Guessing provider APIs
* Adding dependencies for trivial functionality
* Changing public APIs without considering compatibility

Do not rewrite an entire file when a localized change is sufficient.

## Provider development

Providers are external systems and may change without notice.

### Verify the breakage before fixing it

Before modifying a provider implementation, confirm that the reported
failure is still reproducible.

Provider behavior can change independently of ani-cli-rs. A provider may
temporarily break, change its API, or restore previous behavior without any
change to this repository.

If a previously reported failure is no longer reproducible:

* Investigate whether the provider changed or recovered
* Compare the current behavior with the known failing behavior when possible
* Test multiple representative cases
* Do not make speculative code changes solely because a previous failure
  was reported
* If no code change is currently necessary, document the finding instead

Do not manufacture a fix for a provider that has already recovered.

### Fixing provider breakages

When a provider breakage is confirmed:

1. Reproduce the failure.
2. Inspect the current provider behavior.
3. Determine what changed, where reasonably possible.
4. Compare the provider behavior with the existing implementation.
5. Make the smallest practical fix.
6. Test the affected functionality.
7. Test related functionality when the change could affect it.

### Agentic provider investigation

Agents are explicitly allowed to perform extensive provider investigation when necessary.

This may include:

* Browsing provider websites
* Inspecting publicly accessible APIs
* Inspecting browser/network requests
* Examining redirects and response headers
* Inspecting JavaScript-driven requests
* Testing provider endpoints
* Testing multiple anime IDs or episodes
* Inspecting HLS manifests
* Running `curl`, `ffmpeg`, `yt-dlp`, or ani-cli-rs
* Reproducing failures automatically
* Comparing multiple provider responses
* Building temporary scripts for investigation
* Running repeated tests against a reasonable sample set

Use this capability to gather evidence rather than guessing.

Provider testing must remain reasonable:

* Do not intentionally overload providers.
* Do not perform denial-of-service or stress testing.
* Avoid unnecessarily large crawls.
* Prefer small representative test sets.
* Use delays or caching where appropriate.
* Do not obtain credentials or private user data.
* Respect authentication and access-control boundaries.

Temporary investigation scripts do not need to become part of the final project unless they provide lasting value.

## Do not confuse investigation with authorization

The fact that an agent can technically access or inspect something does not mean it should.

Do not:

* Attempt to obtain private credentials
* Access another user's private data
* Circumvent authentication
* Circumvent access controls
* Perform destructive actions against external systems
* Intentionally degrade or disrupt a provider

When investigation encounters a security boundary, stop and reassess rather than attempting to bypass it.

## Testing

After making changes, run the narrowest relevant checks first.

Then run broader checks when practical.

At minimum, consider:

```text
cargo check
cargo test
cargo fmt --check
cargo clippy
```

Use the project's documented toolchain and commands rather than assuming the newest Rust version is appropriate.

For provider changes, local compilation is not sufficient. Test the affected provider behavior against the current upstream service when reasonably possible.

Do not claim that something was tested if it was not actually tested.

## Error handling

User-facing errors should be:

* Clear
* Actionable
* Consistent with existing error handling
* Appropriate for the CLI

Do not expose raw internal errors when a useful user-facing explanation is available.

When adding or changing errors, preserve the project's error architecture and localization conventions.

## Dependencies

Before adding a dependency:

1. Check whether the functionality already exists in the standard library or an existing dependency.
2. Check whether an existing project abstraction can solve the problem.
3. Consider cross-platform support.
4. Consider maintenance cost and dependency size.
5. Check the dependency's license and compatibility.

Do not add a dependency solely because an AI suggested it.

## Public APIs

Treat the `ani_cli` library API as a public interface.

Before changing public types, functions, modules, or behavior:

* Search for internal usages.
* Consider downstream consumers.
* Preserve compatibility where practical.
* Avoid breaking changes unless there is a clear reason.

## Documentation

Update documentation when behavior, commands, options, provider configuration, or public APIs change.

Documentation should describe verified behavior.

Do not invent commands, options, URLs, provider behavior, or configuration fields.

## Git and pull requests

Keep changes focused.

Do not:

* Modify unrelated files
* Reformat unrelated code
* Rewrite history
* Create commits unless explicitly requested
* Push changes unless explicitly requested
* Open or merge pull requests autonomously

Before considering the work complete, inspect the final diff.

Ask:

* Is every changed line necessary?
* Did I accidentally modify unrelated code?
* Does the implementation match existing project conventions?
* Did I test the actual behavior?
* Are documentation changes included where necessary?

## AI-specific requirements

AI-generated code is untrusted input.

Do not blindly accept generated implementations.

A human contributor must understand, review, verify, and submit the final changes.

For substantial AI-generated content, follow [`AI_POLICY.md`](AI_POLICY.md).

Provider investigation is an explicit exception to the normal expectation of limited agent activity: agents may browse, scrape, probe, and test external providers extensively when necessary to diagnose a real provider problem, subject to the responsible-testing rules above.

## When uncertain

Prefer investigation over guessing.

Search the repository.
Read the relevant documentation.
Inspect the actual provider behavior.
Run a focused test.

If uncertainty remains about an architectural or behavioral decision, explain the uncertainty rather than silently choosing an assumption.
