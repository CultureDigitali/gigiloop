import { describe, expect, it } from "vitest";
import { reconcileMidiSelection } from "./deviceSelection";

const ports = [
  { id: "midir:0:Keyboard", index: 0, name: "Keyboard" },
  { id: "midir:1:MPD24", index: 1, name: "MPD24" }
];

describe("reconcileMidiSelection", () => {
  it("keeps a selection that is still available", () => {
    expect(reconcileMidiSelection("midir:1:MPD24", ports)).toBe("midir:1:MPD24");
  });

  it("clears a stale selection when multiple ports remain", () => {
    expect(reconcileMidiSelection("midir:9:MPD24", ports)).toBe("");
  });

  it("selects the only available port after reconnect", () => {
    expect(
      reconcileMidiSelection("midir:9:MPD24", [
        { id: "midir:0:MPD24", index: 0, name: "MPD24" }
      ])
    ).toBe("midir:0:MPD24");
  });
});
