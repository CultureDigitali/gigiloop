# Evidence-tier scoring

Use evidence tiers to prevent confidence from masquerading as verification.

## Score anchors

- **0–4:** missing, broken, or materially wrong.
- **5–6:** partly works, inspection-only, fragile, or substantially incomplete.
- **7–8:** works for verified paths but lacks important integration, adversarial, or regression evidence.
- **9:** solid, current, evidence-backed, and free of known defects above nitpick level.
- **10:** exceptional; all relevant evidence is current, the final gate is clean, and an independent or genuinely fresh-context review found no material issue.

## Evidence tiers and practical ceilings

| Tier | Evidence | Typical ceiling |
|---|---|---:|
| **T0** | intuition, unsupported claim, or stale evidence | 4/10 |
| **T1** | static inspection only | 6/10 |
| **T2** | deterministic static checks such as lint, typecheck, build | 7/10 |
| **T3** | targeted automated test or reproducible behavior | 8/10 |
| **T4** | T3 + adversarial edge cases + relevant integration/regression verification | 9/10 |
| **T5** | T4 + independent/fresh-context review + clean final gate + integrity checks | 10/10 |

The ceiling is a constraint, not a target. Weak coverage, known uncertainty, or a material finding may require a lower score even at a high tier.

## Current-evidence rule

Evidence must identify the code state it verifies. When relevant implementation, tests, snapshots, thresholds, or configuration change:

1. mark affected evidence stale;
2. lower scores that depended on it when necessary;
3. re-run the smallest sufficient checks;
4. use only current evidence at the final gate.

## Integrity cap

A criterion cannot pass while any of these remains unresolved:

- verification was weakened to obtain green output;
- a confirmed regression remains;
- protected user work was overwritten or entangled without acceptance;
- a critical acceptance criterion was redefined to fit the implementation;
- a claimed check was not actually run;
- a high-severity adversarial finding remains open.

## Weighted score

Track weighted totals for progress, but do not use the average as the completion gate. Every critical criterion must independently pass.

## Reconciliation

After red-team review:

- confirmed findings lower the relevant criterion;
- falsified findings cite the evidence that rejected them;
- hypotheses remain uncertainty until tested;
- post-reconciliation scores become authoritative.
