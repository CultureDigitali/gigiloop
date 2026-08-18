# Security

GigiLoop is an instruction skill that can cause an agent to edit code, run tests, and inspect repositories. Treat it with the same care as any autonomous coding workflow.

## Reporting a security issue

Do not publish secrets, credentials, private source code, customer data, or exploit details in a public issue. Provide a minimal redacted reproduction and contact the maintainers privately through the repository owner's published contact channels when necessary.

## Safety expectations

- Respect repository and host permission boundaries.
- Never weaken security controls merely to make a test pass.
- Treat secrets found during review as sensitive data and avoid echoing them into logs or checkpoints.
- Prefer read-only permissions for independent reviewers when the host supports them.
- Do not execute untrusted project scripts without considering repository context and user authorization.
- Record external blockers rather than bypassing access controls.
