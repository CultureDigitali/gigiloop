import type { MidiEvent } from "./types";

export const MAX_EVENTS = 500;

export function appendMidiEvent(
  current: MidiEvent[],
  event: MidiEvent,
  limit = MAX_EVENTS
): MidiEvent[] {
  if (limit <= 0) {
    return [];
  }

  return [...current, event].slice(-limit);
}
