import type { MidiPortInfo } from "../types";

type Props = {
  ports: MidiPortInfo[];
  selectedId: string;
  connected: boolean;
  busy: boolean;
  onSelect: (id: string) => void;
  onRefresh: () => void;
  onConnect: () => void;
  onDisconnect: () => void;
};

export function DevicePicker(props: Props) {
  return (
    <section className="panel device-picker">
      <div className="panel-title">MIDI Input</div>
      <div className="row">
        <select
          value={props.selectedId}
          disabled={props.busy || props.connected}
          onChange={(event) => props.onSelect(event.target.value)}
        >
          <option value="">Select a MIDI input</option>
          {props.ports.map((port) => (
            <option key={port.id} value={port.id}>
              {port.name}
            </option>
          ))}
        </select>
        <button disabled={props.busy} onClick={props.onRefresh}>
          Refresh
        </button>
        {props.connected ? (
          <button disabled={props.busy} onClick={props.onDisconnect}>
            Disconnect
          </button>
        ) : (
          <button
            disabled={props.busy || !props.selectedId}
            onClick={props.onConnect}
          >
            Connect
          </button>
        )}
      </div>
    </section>
  );
}
