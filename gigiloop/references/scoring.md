# Scoring and evidence tiers

Use scores as a compact summary of verified quality, not as a substitute for verification.

## Evidence tiers

| Tier | Evidence | Typical ceiling |
|---|---|---:|
| T0 | intuition, unsupported self-assessment | 4/10 |
| T1 | static inspection or reasoning only | 6/10 |
| T2 | deterministic static checks: lint, typecheck, build, schema validation | 7/10 |
| T3 | targeted automated tests or a reproducible behavioral check | 8/10 |
| T4 | T3 + adversarial/edge-case tests + relevant integration verification | 9/10 |
| T5 | T4 + fresh-context/independent review + clean final gate | 10/10 |

Treat these as ceilings, not automatic scores. A test passing does not imply an 8 if important paths remain untested.

## Anchors

- **0–4:** missing, broken, contradicted by evidence, or not meaningfully verified.
- **5–6:** partially works or is plausible but fragile, incomplete, or inspection-only.
- **7:** deterministic checks pass, but behavioral proof is incomplete.
- **8:** targeted behavior is verified and meaningful regression coverage exists, with material uncertainty remaining.
- **9:** core and edge behavior is verified, relevant integration risk is covered, and no known material defect remains.
- **10:** 9-level evidence plus independent/fresh-context review and a clean final gate; no material unresolved uncertainty remains.

## Reconciliation rule

The score sheet is provisional until adversarial findings are reconciled.

- A confirmed material finding must reduce the affected score if the prior score is no longer justified.
- A falsified finding must cite the evidence that disproved it.
- An untested hypothesis remains uncertainty, not a defect.

Never change the rubric or evidence threshold merely to make current work pass.
