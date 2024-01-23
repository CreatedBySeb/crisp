use std::f32::consts::PI;

use sdl2::{
    audio::{AudioCallback, AudioDevice, AudioSpecDesired},
    AudioSubsystem,
};

const SAMPLING_RATE: f32 = 44100.0;
const TWO_PI: f32 = 2.0 * PI;

const AUDIO_SPEC: AudioSpecDesired = AudioSpecDesired {
    freq: Some(SAMPLING_RATE as i32),
    channels: Some(1),
    samples: None,
};

pub struct Tone {
    frequency: f32,
    index: f32,
    volume: f32,
}

impl AudioCallback for Tone {
    type Channel = f32;

    fn callback(&mut self, out: &mut [Self::Channel]) {
        for x in out.iter_mut() {
            let sine_value = self.index.sin();
            *x = self.volume * (if sine_value >= 0.0 { 1.0 } else { -1.0 });
            self.index += (self.frequency * TWO_PI) / SAMPLING_RATE;

            if self.index >= TWO_PI {
                self.index -= TWO_PI;
            }
        }
    }
}

pub fn get_audio_device(subsystem: AudioSubsystem) -> AudioDevice<Tone> {
    subsystem
        .open_playback(None, &AUDIO_SPEC, |_| Tone {
            frequency: 440.0,
            index: 0.0,
            volume: 0.2,
        })
        .unwrap()
}
