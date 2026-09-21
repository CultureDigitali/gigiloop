import { describe, expect, it } from "vitest";
import { padForKeyboardKey } from "./virtualPads";

describe("padForKeyboardKey", () => {
  it("maps the 4x4 keyboard layout to pads", () => {
    expect(padForKeyboardKey("1")).toBe(0);
    expect(padForKeyboardKey("4")).toBe(3);
    expect(padForKeyboardKey("Q")).toBe(4);
    expect(padForKeyboardKey("v")).toBe(15);
  });

  it("ignores unrelated keys", () => {
    expect(padForKeyboardKey("Enter")).toBeNull();
  });
});
