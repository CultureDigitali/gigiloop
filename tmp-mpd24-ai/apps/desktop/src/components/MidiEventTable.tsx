import type { MidiEvent } from "../types";

type Props = {
  events: MidiEvent[];
};

export function MidiEventTable({ events }: Props) {
  return (
    <section className="panel event-panel">
      <div className="panel-title">Live MIDI Events</div>
      <div className="event-scroll">
        <table>
          <thead>
            <tr>
              <th>Time μs</th>
              <th>Channel</th>
              <th>Type</th>
              <th>Data 1</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            {events.map((event, index) => (
              <tr key={`${event.timestampMicros}-${index}`}>
                <td>{event.timestampMicros}</td>
                <td>{event.channel === 0 ? "—" : event.channel}</td>
                <td>{event.kind}</td>
                <td>{event.data1}</td>
                <td>{event.value.value}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}
