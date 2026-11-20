# Security policy

## Reporting a vulnerability

Report security problems through
[GitHub's private vulnerability reporting](https://github.com/gaborbernat/simplint/security/advisories/new). If you do
not have a GitHub account, email [gaborjbernat@gmail.com](mailto:gaborjbernat@gmail.com).

Only the latest release receives fixes. Confirm the problem on that release and include the platform, the input the
attacker controls, the steps to reproduce it and the impact. Verify any tool output you include and describe the finding
in your own words.

## What to expect

A volunteer maintains simplint without an on-call rotation. We aim to acknowledge reports within **5 working days** and
assess them within **15 working days**. We develop fixes in a private fork attached to a draft advisory, request a CVE
through GitHub and publish the advisory with the fixed release. The default embargo is **90 days** from the report. We
agree on any change with the reporter.

You can choose how we credit you in the advisory or ask to remain anonymous. There is no bug bounty.

## Vulnerability or bug

The [threat model](THREAT_MODEL.md) explains which inputs an attacker can control. Report a problem privately if one of
those inputs can cause any of the following during linting.

- **High** — Execute code, make a network request, or read or write a file outside the permitted paths.
- **Medium** — Hang simplint or exhaust memory with a small input.
- **Low** — Crash or panic on malformed input.

Use a public issue for incorrect diagnostics and fixes that change program behavior unexpectedly. Unsafe fixes can
change behavior by design. A clean error on malformed input is expected.

A finding that requires control of the caller's machine, environment, command-line options or chosen paths also belongs
in a public issue. Naming a symlink on the command line includes its target in the run. When walking a directory,
simplint skips file symlinks that lead outside all the chosen paths.

If you are unsure how to classify a finding, report it privately.
