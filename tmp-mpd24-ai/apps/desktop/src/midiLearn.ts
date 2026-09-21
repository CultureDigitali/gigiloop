import type { MidiEvent, MidiEventKind } from "./types";

export type LearnedMapping = {
  controlId: string;
  messageKind: MidiEventKind;
  channel: number;
  data1: number;
  label: string;
};

export function learnMapping(
  target: string,
  event: MidiEvent
): LearnedMapping | null {
  if (!target || event.kind === "noteOff") {
    return null;
  }

  return {
    controlId: target,
    messageKind: event.kind,
    channel: event.channel,
    data1: event.data1,
    label: target
  };
}

export function upsertLearnedMapping(
  mappings: LearnedMapping[],
  mapping: LearnedMapping
): LearnedMapping[] {
  return [...mappings.filter((item) => item.controlId !== mapping.controlId), mapping];
}
