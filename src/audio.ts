import { invoke } from "@tauri-apps/api/core";

export type PlayMode = "both" | "speakers" | "microphone";

export type AudioStatus = {
  mode: PlayMode;
  speakerDevice: string;
  micDevice: string;
  speakerChoice: string;
  micChoice: string;
  devices: string[];
  playing: string[];
  hint: string;
};

export function loadAudio(): Promise<AudioStatus> {
  return invoke<AudioStatus>("audio_status");
}

export function saveAudio(
  mode: PlayMode,
  speakerDevice: string,
  micDevice: string,
): Promise<AudioStatus> {
  return invoke<AudioStatus>("set_audio", { mode, speakerDevice, micDevice });
}

export function playSound(id: string, mode?: PlayMode): Promise<AudioStatus> {
  return invoke<AudioStatus>("play_sound", { id, mode });
}

export function stopSounds(): Promise<AudioStatus> {
  return invoke<AudioStatus>("stop_sounds");
}
