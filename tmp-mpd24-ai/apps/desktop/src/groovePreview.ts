import type { SequencerPattern } from "./types";

export function activeHitCount(pattern: SequencerPattern): number {
  return pattern.tracks.reduce(
    (sum, track) => sum + track.steps.filter((step) => step.active).length,
    0
  );
}

export function changedStepCount(
  source: SequencerPattern,
  candidate: SequencerPattern
): number {
  let changed = 0;
  const trackCount = Math.min(source.tracks.length, candidate.tracks.length);
  for (let track = 0; track < trackCount; track += 1) {
    const stepCount = Math.min(
      source.tracks[track].steps.length,
      candidate.tracks[track].steps.length
    );
    for (let step = 0; step < stepCount; step += 1) {
      const before = source.tracks[track].steps[step];
      const after = candidate.tracks[track].steps[step];
      if (
        before.active !== after.active ||
        before.velocity !== after.velocity ||
        before.microOffsetMicros !== after.microOffsetMicros
      ) {
        changed += 1;
      }
    }
  }
  return changed;
}
