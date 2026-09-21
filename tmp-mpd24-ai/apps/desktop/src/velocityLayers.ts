import type { VelocityLayerSnapshot } from "./types";

export const MAX_VELOCITY_LAYERS = 8;
export const MAX_ROUND_ROBIN_VARIANTS = 8;

export function cloneVelocityLayers(
  layers: VelocityLayerSnapshot[]
): VelocityLayerSnapshot[] {
  return layers.map((layer) => ({
    ...layer,
    sampleFiles: [...layer.sampleFiles]
  }));
}

export function validateVelocityLayers(
  layers: VelocityLayerSnapshot[]
): string | null {
  if (layers.length > MAX_VELOCITY_LAYERS) {
    return `Maximum ${MAX_VELOCITY_LAYERS} layers per pad.`;
  }

  for (let index = 0; index < layers.length; index += 1) {
    const layer = layers[index];
    if (
      !Number.isInteger(layer.minVelocity) ||
      !Number.isInteger(layer.maxVelocity) ||
      layer.minVelocity < 1 ||
      layer.maxVelocity > 127 ||
      layer.maxVelocity < layer.minVelocity
    ) {
      return "Every layer needs an integer velocity range from 1 to 127.";
    }
    if (layer.sampleFiles.length === 0) {
      return "Every layer needs at least one WAV.";
    }
    if (layer.sampleFiles.length > MAX_ROUND_ROBIN_VARIANTS) {
      return `Maximum ${MAX_ROUND_ROBIN_VARIANTS} round-robin variants per layer.`;
    }
    if (layer.sampleFiles.some((file) => file.trim().length === 0)) {
      return "Sample references cannot be empty.";
    }
    if (new Set(layer.sampleFiles).size !== layer.sampleFiles.length) {
      return "The same WAV cannot appear twice in one layer.";
    }

    for (let otherIndex = index + 1; otherIndex < layers.length; otherIndex += 1) {
      const other = layers[otherIndex];
      const overlaps =
        layer.minVelocity <= other.maxVelocity &&
        other.minVelocity <= layer.maxVelocity;
      if (overlaps) {
        return "Velocity ranges cannot overlap.";
      }
    }
  }

  return null;
}

export function firstFreeVelocityRange(
  layers: VelocityLayerSnapshot[]
): { minVelocity: number; maxVelocity: number } | null {
  const occupied = Array.from({ length: 128 }, () => false);
  for (const layer of layers) {
    for (
      let velocity = Math.max(1, Math.trunc(layer.minVelocity));
      velocity <= Math.min(127, Math.trunc(layer.maxVelocity));
      velocity += 1
    ) {
      occupied[velocity] = true;
    }
  }

  let start = 1;
  while (start <= 127 && occupied[start]) {
    start += 1;
  }
  if (start > 127) {
    return null;
  }

  let end = start;
  while (end < 127 && !occupied[end + 1]) {
    end += 1;
  }
  return { minVelocity: start, maxVelocity: end };
}

export function createVelocityPreset(
  zones: 2 | 3
): VelocityLayerSnapshot[] {
  if (zones === 2) {
    return [
      {
        minVelocity: 1,
        maxVelocity: 72,
        sampleFiles: [],
        roundRobin: false
      },
      {
        minVelocity: 73,
        maxVelocity: 127,
        sampleFiles: [],
        roundRobin: false
      }
    ];
  }

  return [
    {
      minVelocity: 1,
      maxVelocity: 45,
      sampleFiles: [],
      roundRobin: false
    },
    {
      minVelocity: 46,
      maxVelocity: 95,
      sampleFiles: [],
      roundRobin: false
    },
    {
      minVelocity: 96,
      maxVelocity: 127,
      sampleFiles: [],
      roundRobin: false
    }
  ];
}

export function velocityLayerIndexFor(
  layers: VelocityLayerSnapshot[],
  velocity: number
): number | null {
  if (!Number.isInteger(velocity) || velocity < 1 || velocity > 127) {
    return null;
  }
  const index = layers.findIndex(
    (layer) =>
      velocity >= layer.minVelocity &&
      velocity <= layer.maxVelocity &&
      layer.sampleFiles.length > 0
  );
  return index >= 0 ? index : null;
}

export function velocityCoverageGaps(
  layers: VelocityLayerSnapshot[]
): Array<{ minVelocity: number; maxVelocity: number }> {
  const covered = Array.from({ length: 128 }, () => false);
  for (const layer of layers) {
    if (
      !Number.isInteger(layer.minVelocity) ||
      !Number.isInteger(layer.maxVelocity)
    ) {
      continue;
    }
    const start = Math.max(1, layer.minVelocity);
    const end = Math.min(127, layer.maxVelocity);
    if (end < start || layer.sampleFiles.length === 0) {
      continue;
    }
    for (let velocity = start; velocity <= end; velocity += 1) {
      covered[velocity] = true;
    }
  }

  const gaps: Array<{ minVelocity: number; maxVelocity: number }> = [];
  let velocity = 1;
  while (velocity <= 127) {
    if (covered[velocity]) {
      velocity += 1;
      continue;
    }
    const start = velocity;
    while (velocity < 127 && !covered[velocity + 1]) {
      velocity += 1;
    }
    gaps.push({ minVelocity: start, maxVelocity: velocity });
    velocity += 1;
  }
  return gaps;
}
