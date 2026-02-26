# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in swapx, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please email the maintainer directly or use [GitHub's private vulnerability reporting](https://github.com/BeepBoopBit/swapx/security/advisories/new).

### What to include

- A description of the vulnerability
- Steps to reproduce the issue
- The potential impact
- Any suggested fixes (optional)

### What to expect

- Acknowledgment within 48 hours
- A status update within 7 days
- We will work with you to understand and address the issue before any public disclosure

## Security Considerations

swapx executes shell commands on your behalf. Keep these points in mind:

- **Review your rules carefully.** swapx transforms commands exactly as configured. A misconfigured rule could rewrite a command in unintended ways.
- **Protect your config files.** `.swapx.yaml` and `~/.config/swapx/rules.yaml` control what commands get rewritten. Ensure they have appropriate file permissions.
- **Be cautious with regex rules.** Overly broad regex patterns could match and rewrite commands you didn't intend.
- **Shell hooks execute automatically.** When shell integration is enabled, swapx intercepts commands before execution. Use `SWAPX_AUTO_APPLY=1` only when you trust all configured rules.
