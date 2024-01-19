use sdl2::{
    audio::{AudioCallback, AudioDevice, AudioSpecDesired},
    AudioSubsystem,
};

const AUDIO_SPEC: AudioSpecDesired = AudioSpecDesired {
    freq: Some(44100),
    channels: Some(1),
    samples: None,
};

pub struct Tone {
    volume: f32,
}

impl AudioCallback for Tone {
    type Channel = f32;

    fn callback(&mut self, out: &mut [Self::Channel]) {
        for x in out.iter_mut() {
            *x = self.volume;
        }
    }
}

pub fn get_audio_device(subsystem: AudioSubsystem) -> AudioDevice<Tone> {
    subsystem
        .open_playback(None, &AUDIO_SPEC, |spec| Tone { volume: 0.25 })
        .unwrap()
}
