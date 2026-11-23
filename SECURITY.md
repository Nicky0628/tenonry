# Security Policy

## Supported Versions

Security fixes are provided for the latest minor release line:

| Version | Supported |
| --- | --- |
| `0.1.x` | Yes |

## Security Model & Boundaries

Tenonry coordinates host package managers (`APT`, `DNF`, `pacman`, `WinGet`, `npm`) and may invoke administrative elevation (such as `pkexec` on Linux or UAC-elevated package manager steps on Windows). To minimize risk:

- Commands are constructed with explicit argument arrays rather than shell string interpolation.
- Plans lock target packages and versions prior to execution and re-verify live system state before launching commands.
- System-protected components, mixed-source installations, and multi-backend conflicts are blocked from automatic modification.
- The standalone desktop app restricts IPC invocations to its bundled main window origin and enforces a strict Content Security Policy (CSP).

## Reporting a Vulnerability

Please **do not** report security vulnerabilities through public GitHub issues or pull requests.

Instead, report vulnerabilities privately via GitHub Security Advisories:

- <https://github.com/Nicky0628/tenonry/security/advisories/new>

When reporting, please include:

- Affected component (`tenonry` library or `tenonry-desktop`) and version
- Operating system, architecture, and relevant package manager versions
- Minimal reproduction steps or proof of concept
- Expected impact and any suggested remediation

We aim to acknowledge reports within 72 hours and coordinate a fix before public disclosure.

---

## 安全漏洞报告（中文）

本项目涉及调用系统包管理器与提权执行。如发现安全漏洞，请**勿**在公开 Issue 中披露，请通过 GitHub Security Advisories 私密提交：

- <https://github.com/Nicky0628/tenonry/security/advisories/new>
