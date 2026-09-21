import { describe, expect, it } from "vitest";
import { activeHitCount, changedStepCount } from "./groovePreview";
import type { SequencerPattern } from "./types";

function pattern(active: boolean): SequencerPattern {
  return {
    name: "Test",
    bpm: 100,
    totalSteps: 1,
    swing: 0,
    tracks: [
      {
        pad: 0,
        length: 1,
        muted: false,
        steps: [
          {
            active,
            velocity: 100,
            probability: 1,
            microOffsetMicros: 0
          }
        ]
      }
    ]
  };
}

describe("groove preview helpers", () => {
  it("counts active hits", () => {
    expect(activeHitCount(pattern(true))).toBe(1);
    expect(activeHitCount(pattern(false))).toBe(0);
  });

  it("counts changed steps", () => {
    expect(changedStepCount(pattern(false), pattern(true))).toBe(1);
    expect(changedStepCount(pattern(true), pattern(true))).toBe(0);
  });
});
