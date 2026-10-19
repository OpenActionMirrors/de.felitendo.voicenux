import { RandomSound, SoundVolume, SoundVolumeDial, Sound, StopSounds } from "./sounds.js";
import { Background, HearMyself, HearSounds, Mute, VoiceChanger } from "./switches.js";
import { NextVoice, RandomVoice, Voice, VoiceDial } from "./voices.js";

export const ACTIONS = [Voice, Sound, VoiceChanger, Mute, HearMyself, Background, RandomVoice, NextVoice, StopSounds, RandomSound, SoundVolume, HearSounds, VoiceDial, SoundVolumeDial];
