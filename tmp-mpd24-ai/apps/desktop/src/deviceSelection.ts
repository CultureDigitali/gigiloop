import type { MidiPortInfo } from "./types";

export function reconcileMidiSelection(
  selectedId: string,
  ports: MidiPortInfo[]
): string {
  if (selectedId && ports.some((port) => port.id === selectedId)) {
    return selectedId;
  }

  if (ports.length === 1) {
    return ports[0].id;
  }

  return "";
}
