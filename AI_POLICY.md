# AI Contribution Policy

ani-cli-rs allows the use of AI-assisted development tools, including code completion tools, chat-based assistants, and coding agents.

AI is a tool, not a contributor. **Every contribution must ultimately be reviewed, understood, tested, and submitted by a human.**

## 1. AI-assisted contributions are allowed

You may use AI to assist with:

* Writing or refactoring code
* Understanding unfamiliar code
* Debugging
* Writing tests
* Improving documentation
* Translating documentation
* Finding potential edge cases
* Generating ideas or implementation approaches
* Reviewing your own changes

Using AI does not automatically make a contribution unacceptable.

## 2. You are responsible for what you submit

If you use AI, you are still the author and maintainer of your contribution.

Before opening a pull request, you must:

* Understand the changes you are submitting
* Review the generated output rather than blindly accepting it
* Verify that the implementation actually works
* Run the relevant tests and checks
* Make sure the change follows the project's existing architecture and conventions
* Check that dependencies, code, documentation, and other assets can legally be contributed

> "The AI generated it" is not an acceptable explanation for a contribution you do not understand.

Maintainers may ask you to explain implementation decisions or modify code during review.

## 3. Provider breakages and agentic investigation

ani-cli-rs depends on external providers and APIs that may change or break without notice. When an upstream change causes searching, playback, source extraction, subtitles, or downloading to stop working, **investigating and fixing the breakage is considered a time-sensitive maintenance task.**

AI coding agents may be used extensively for this work. This includes agents capable of browsing websites, inspecting network traffic, executing commands, interacting with provider pages, and performing automated tests.

For provider breakages, an agent may:

* Investigate the provider's current website and publicly accessible APIs
* Crawl relevant provider pages to determine their current structure
* Inspect HTTP requests, responses, redirects, headers, and network behavior
* Inspect JavaScript-driven requests and browser/DevTools network activity
* Test search, episode, source, subtitle, and download endpoints
* Test multiple anime IDs, episodes, languages, providers, or quality options
* Inspect HLS manifests and related media requests
* Run `curl`, `ffmpeg`, `yt-dlp`, or ani-cli-rs commands against the provider
* Reproduce reported failures
* Compare current provider behavior against the existing implementation
* Automatically test multiple cases to determine the scope of a breakage
* Identify likely upstream changes
* Implement and test a focused fix
* Run the relevant project tests and checks

This is intentionally allowed to be **more extensive than ordinary AI-assisted development**. Provider maintenance often requires interacting with systems that cannot be understood reliably from the source code alone.

### Responsible provider testing

Agentic investigation must remain reasonable and should not unnecessarily burden external providers.

Contributors should:

* Prefer publicly accessible endpoints and normal user-facing behavior
* Use reasonable request volumes
* Prefer small test sets when they are sufficient to reproduce a problem
* Use caching, delays, or other measures where appropriate
* Avoid denial-of-service, stress testing, or deliberate resource exhaustion
* Respect authentication boundaries and access controls
* Never attempt to obtain credentials, private information, or another user's data
* Avoid circumventing access controls or security mechanisms unless explicitly authorized for legitimate security research
* Stop automated testing if continued activity could reasonably harm or disrupt the provider

### Human verification

An agent may perform the investigation and prepare a complete fix, but the human contributor remains responsible for the final result.

Before submitting a provider-related pull request, the contributor should verify:

1. **What is broken** - the observed failure should be reproducible or otherwise supported by evidence.
2. **What changed** - identify the relevant upstream change where reasonably possible.
3. **Why the fix works** - understand the relationship between the provider change and the proposed implementation.
4. **The scope of the fix** - avoid unrelated refactors or speculative provider changes.
5. **Compatibility** - verify that the fix does not unnecessarily break other provider functionality.
6. **Testing** - confirm the affected functionality works against the current provider.

The agent may gather evidence, perform repetitive testing, and iterate quickly. **It does not replace human judgment about what should be committed.**

### Provider fixes may receive review priority

A provider fix that restores currently broken user-facing functionality may receive priority during maintainer review.

This priority is based on the **impact and urgency of the provider breakage**, not on whether AI was used.

For example, a small agent-assisted change that updates an endpoint after a provider migration may reasonably be reviewed ahead of a larger refactor that does not address a current breakage.

Contributors should keep urgent provider fixes focused. Do not use the urgency of a provider breakage as justification for unrelated changes.

> **For provider maintenance, agents may investigate deeply and test broadly. Humans must still understand, verify, review, and submit the final fix.**

## 4. AI-generated code must be reviewed

AI-generated code should be treated as **untrusted input**.

Do not assume that generated code is:

* Correct
* Secure
* Efficient
* Compatible with the project
* Compatible with the Rust version or dependencies in use
* Free from licensing or copyright concerns

Compilation and passing tests are not sufficient by themselves.

Contributors should also consider whether the implementation is appropriate for the project and whether it introduces unnecessary complexity.

## 5. Disclosure

We do not require disclosure for normal AI assistance such as:

* Autocomplete
* Spelling and grammar corrections
* Translation
* Asking an AI to explain existing code
* Brainstorming
* Small suggestions that you independently implement

For **substantial AI-generated content**, disclosure is expected.

Examples include:

* A significant portion of a source file being generated by an AI tool
* A large refactor produced primarily by an AI agent
* Documentation substantially written by an AI system
* A feature where an AI coding agent produced most of the implementation

In these cases, mention the use of AI in the pull request description.

For example:

```text
AI assistance: Claude was used to generate the initial implementation.
The resulting code was reviewed, modified, and tested manually.
```

You do not need to provide prompts, conversation transcripts, or model output unless a maintainer specifically requests additional context.

## 6. Autonomous agents

AI coding agents may be used to assist with development, but they must operate under human supervision.

An AI agent must not independently:

* Open or merge pull requests
* Approve pull requests
* Resolve maintainer review comments without human review
* Submit issue reports
* Participate in project discussions as if it were a human contributor
* Make releases

A human contributor must review and submit the final work.

This does not prevent an agent from performing extensive automated investigation or testing as described in §3.

## 7. Issues and discussions

Please do not submit raw AI-generated output as an issue.

If an AI assistant identifies a potential bug, security problem, or feature idea, investigate it yourself before opening an issue.

Issues should describe an actual, reproducible problem or a clearly understood proposal.

Likewise, do not use AI to generate large numbers of speculative issues, comments, or discussions.

## 8. Security

Never provide private project information, credentials, tokens, secrets, or sensitive user data to an AI service.

AI-generated security fixes require particularly careful review and testing.

If you discover a potential security vulnerability, follow [`SECURITY.md`](SECURITY.md) rather than publicly posting AI-generated analysis.

## 9. Licensing and provenance

Contributors are responsible for ensuring that their contributions can legally be submitted to ani-cli-rs.

Do not intentionally ask an AI system to reproduce code from a specific copyrighted project or source.

Be particularly careful when an AI tool produces code that appears unusually similar to existing third-party code.

AI-generated output does not remove the contributor's responsibility to comply with:

* The GPL-3.0-only license
* Third-party licenses
* Copyright law
* The terms of relevant AI tools
* Any other applicable project requirements

If you cannot establish that generated content is appropriate to contribute, do not submit it.

## 10. Maintainer discretion

Maintainers may request changes, additional testing, disclosure, or clarification when AI-assisted work makes a contribution difficult to review.

Pull requests may be closed if they are:

* Clearly untested
* Largely unreviewed AI output
* Excessively verbose or unnecessarily complex
* Based on hallucinated project APIs or behavior
* Submitted without understanding the existing architecture
* Submitted in large quantities without meaningful human review
* Based on speculative provider changes without evidence of an underlying provider change or breakage

This policy is about **maintaining contribution quality, not detecting or banning AI**.

A well-tested, well-understood contribution is preferable to an unreviewed AI contribution regardless of which tools were used to create it.

## 11. The standard

The simplest rule is:

> **Use whatever tools help you build. Understand, verify, and take responsibility for whatever you submit.**

AI-assisted development is welcome. Unreviewed AI output is not.
