import type { MidiStatus } from "../types";

export function StatusStrip({ status }: { status: MidiStatus }) {
  return (
    <div className={`status-strip ${status.connected ? "connected" : "disconnected"}`}>
      <strong>{status.connected ? "CONNECTED" : "DISCONNECTED"}</strong>
      <span>{status.port?.name ?? "No MIDI input"}</span>
      <span>{status.message}</span>
    </div>
  );
}
