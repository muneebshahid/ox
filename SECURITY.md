# Security Policy

## Supported Versions

This project is pre-1.0 and does not currently guarantee backported security fixes to older tags.
Security fixes are made on the latest default branch first.

## Reporting a Vulnerability

Please report vulnerabilities privately by emailing:

- muneebshahid5@gmail.com

Do not open public GitHub issues for potential security vulnerabilities.

When reporting, include:

- affected version or commit
- impact assessment
- reproduction steps or proof of concept
- any suggested mitigations

You can expect an acknowledgment within 72 hours.

## Scope Notes

`ox` includes a `bash` tool that can execute shell commands. This behavior is intentional,
but unsafe default configurations, privilege escalation paths, or sandbox bypasses are in scope
for security reports.
