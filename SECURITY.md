# Security policy

## Supported versions

Only the latest release receives fixes.

## Reporting a vulnerability

Please **do not open a public issue** for security problems. Use GitHub's
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
on this repository ("Security" tab → "Report a vulnerability").

Please include what you found, how to reproduce it, and the impact. You will get a reply as soon as possible; fixes are
released together with a public advisory once users can update.

## Scope notes

- The app runs locally and moves files only after user approval. Issues where a crafted file name or folder layout
  makes the app move, overwrite or delete something the user did not approve are in scope and high priority.
- The model download is pinned to one revision and verified by SHA-256; a way around that check is in scope.
- The `_Karantina` detection of suspicious PHP is a convenience heuristic, not an antivirus.
