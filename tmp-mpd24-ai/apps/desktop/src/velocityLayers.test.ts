import { describe, expect, it } from "vitest";
import type { VelocityLayerSnapshot } from "./types";
import {
  cloneVelocityLayers,
  createVelocityPreset,
  firstFreeVelocityRange,
  validateVelocityLayers,
  velocityCoverageGaps,
  velocityLayerIndexFor
} from "./velocityLayers";

function layer(
  minVelocity: number,
  maxVelocity: number,
  sampleFiles = ["sample.wav"]
): VelocityLayerSnapshot {
  return {
    minVelocity,
    maxVelocity,
    sampleFiles,
    roundRobin: false
  };
}

describe("velocity-layer helpers", () => {
  it("accepts adjacent non-overlapping ranges", () => {
    expect(
      validateVelocityLayers([layer(1, 63), layer(64, 127)])
    ).toBeNull();
  });

  it("rejects overlapping ranges", () => {
    expect(validateVelocityLayers([layer(1, 80), layer(70, 127)])).toBe(
      "Velocity ranges cannot overlap."
    );
  });

  it("rejects fractional or out-of-range MIDI velocities", () => {
    expect(validateVelocityLayers([layer(1.5, 80)])).not.toBeNull();
    expect(validateVelocityLayers([layer(1, 128)])).not.toBeNull();
  });

  it("rejects duplicate and empty sample references", () => {
    expect(
      validateVelocityLayers([layer(1, 127, ["a.wav", "a.wav"])])
    ).toBe("The same WAV cannot appear twice in one layer.");
    expect(validateVelocityLayers([layer(1, 127, [" "])])).toBe(
      "Sample references cannot be empty."
    );
  });

  it("finds the first free velocity gap", () => {
    expect(
      firstFreeVelocityRange([layer(1, 45), layer(70, 127)])
    ).toEqual({ minVelocity: 46, maxVelocity: 69 });
    expect(firstFreeVelocityRange([layer(1, 127)])).toBeNull();
  });

  it("deep-clones sample file arrays", () => {
    const source = [layer(1, 127, ["a.wav"])];
    const copy = cloneVelocityLayers(source);
    copy[0].sampleFiles.push("b.wav");
    expect(source[0].sampleFiles).toEqual(["a.wav"]);
  });

  it("creates contiguous 2-zone and 3-zone presets", () => {
    const two = createVelocityPreset(2);
    const three = createVelocityPreset(3);
    expect(two.map((item) => [item.minVelocity, item.maxVelocity])).toEqual([
      [1, 72],
      [73, 127]
    ]);
    expect(three.map((item) => [item.minVelocity, item.maxVelocity])).toEqual([
      [1, 45],
      [46, 95],
      [96, 127]
    ]);
  });

  it("selects the matching playable layer for every MIDI velocity", () => {
    const layers = [layer(1, 45), layer(46, 95), layer(96, 127)];
    for (let velocity = 1; velocity <= 127; velocity += 1) {
      const index = velocityLayerIndexFor(layers, velocity);
      if (velocity <= 45) {
        expect(index).toBe(0);
      } else if (velocity <= 95) {
        expect(index).toBe(1);
      } else {
        expect(index).toBe(2);
      }
    }
  });

  it("ignores invalid velocities and empty layers", () => {
    expect(velocityLayerIndexFor([layer(1, 127)], 0)).toBeNull();
    expect(velocityLayerIndexFor([layer(1, 127)], 128)).toBeNull();
    expect(velocityLayerIndexFor([layer(1, 127, [])], 100)).toBeNull();
  });

  it("reports fallback velocity gaps", () => {
    expect(
      velocityCoverageGaps([layer(1, 45), layer(70, 100)])
    ).toEqual([
      { minVelocity: 46, maxVelocity: 69 },
      { minVelocity: 101, maxVelocity: 127 }
    ]);
    expect(velocityCoverageGaps([layer(1, 127)])).toEqual([]);
    expect(velocityCoverageGaps([])).toEqual([
      { minVelocity: 1, maxVelocity: 127 }
    ]);
  });
});
