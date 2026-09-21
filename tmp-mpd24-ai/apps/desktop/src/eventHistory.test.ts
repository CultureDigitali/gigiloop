import { describe, expect, it } from "vitest";
import { appendMidiEvent } from "./eventHistory";

describe("appendMidiEvent", () => {
  it("keeps only the newest events when the history reaches its limit", () => {
    const makeEvent = (timestampMicros: number) => ({
      timestampMicros,
      channel: 1,
      kind: "noteOn" as const,
      data1: 36,
      value: { kind: "sevenBit" as const, value: 100 }
    });

    const initial = [makeEvent(1), makeEvent(2), makeEvent(3)];
    const result = appendMidiEvent(initial, makeEvent(4), 3);

    expect(result.map((event) => event.timestampMicros)).toEqual([2, 3, 4]);
  });
});
