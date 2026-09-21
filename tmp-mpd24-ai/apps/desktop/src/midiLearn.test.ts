import { describe, expect, it } from "vitest";
import { learnMapping, upsertLearnedMapping } from "./midiLearn";

describe("MIDI Learn", () => {
  const noteOn = {
    timestampMicros: 100,
    channel: 10,
    kind: "noteOn" as const,
    data1: 36,
    value: { kind: "sevenBit" as const, value: 110 }
  };

  it("learns the next useful MIDI event for a destination", () => {
    expect(learnMapping("pad-1", noteOn)).toEqual({
      controlId: "pad-1",
      messageKind: "noteOn",
      channel: 10,
      data1: 36,
      label: "pad-1"
    });
  });

  it("ignores note-off as a learn source", () => {
    expect(
      learnMapping("pad-1", {
        ...noteOn,
        kind: "noteOff"
      })
    ).toBeNull();
  });

  it("replaces an existing destination mapping", () => {
    const first = learnMapping("pad-1", noteOn)!;
    const second = { ...first, data1: 42 };
    expect(upsertLearnedMapping([first], second)).toEqual([second]);
  });
});
