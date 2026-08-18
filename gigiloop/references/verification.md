# Verification strategy

Use the cheapest check that can falsify the current change, then broaden verification as confidence grows.

## During an iteration

Prefer:

1. a targeted regression test for the defect or behavior being changed;
2. affected unit tests;
3. affected integration tests when module/API boundaries are involved;
4. relevant lint/typecheck/build checks;
5. manual or UI verification when automated checks cannot cover the behavior.

Do not blindly run the most expensive full suite after every tiny edit if a smaller deterministic check can falsify the change faster.

## Milestones

Run broader verification when:

- multiple files/modules have changed;
- a public interface changed;
- shared infrastructure changed;
- a previously failing criterion reaches 9/10;
- a plateau strategy changes the implementation approach.

## Final gate

Run the complete relevant suite from the cleanest practical state, plus relevant lint/typecheck/build/format checks and a full diff review.

## Failure classification

For every failure classify it as one of:

- `pre-existing` — observed in the baseline before loop changes;
- `regression` — introduced or exposed by the current changes;
- `unknown` — insufficient evidence to assign causality.

Never silently relabel a failing check as pre-existing without baseline evidence.
