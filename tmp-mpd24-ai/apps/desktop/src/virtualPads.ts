export const PAD_LABELS = [
  "Kick",
  "Snare",
  "Closed Hat",
  "Open Hat",
  "Clap",
  "Tom Low",
  "Tom Mid",
  "Tom High",
  "Perc Low",
  "Perc High",
  "Kick Alt",
  "Snare Alt",
  "Hat Alt",
  "Clap Alt",
  "FX A",
  "FX B"
] as const;

export const PAD_KEYS = [
  "1",
  "2",
  "3",
  "4",
  "q",
  "w",
  "e",
  "r",
  "a",
  "s",
  "d",
  "f",
  "z",
  "x",
  "c",
  "v"
] as const;

export function padForKeyboardKey(key: string): number | null {
  const normalized = key.toLowerCase();
  const index = PAD_KEYS.indexOf(normalized as (typeof PAD_KEYS)[number]);
  return index >= 0 ? index : null;
}
