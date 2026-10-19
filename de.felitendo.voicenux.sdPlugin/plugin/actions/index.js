import { SoundVolume, SoundVolumeDial, Sound, StopSounds } from "./sounds.js";
import { Background, HearMyself, HearSounds, Mute, OwnVoice, VoiceChanger } from "./switches.js";
import { NextVoice, RandomVoice, Voice, VoiceDial } from "./voices.js";

export const ACTIONS = [Voice, Sound, VoiceChanger, OwnVoice, Mute, HearMyself, Background, RandomVoice, NextVoice, StopSounds, SoundVolume, HearSounds, VoiceDial, SoundVolumeDial];
