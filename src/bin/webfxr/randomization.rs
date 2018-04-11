use super::*;
use rand::{Rng};

pub fn random_time <G: Rng>(generator: &mut G)->UserTime {
  UserNumber::from_rendered (generator.gen_range (0.001f32.log2(), 3f32.log2()).exp2())
}

pub fn random_waveform <G: Rng>(generator: &mut G)->Waveform {
  match generator.gen_range (0, 4) {
    0 => Waveform::Sine,
    1 => Waveform::Square,
    2 => Waveform::Triangle,
    3 => Waveform::Sawtooth,
    _ => unreachable!(),
  }
}
pub fn random_envelope <G: Rng>(generator: &mut G)->Envelope {
  Envelope {
    attack: UserNumber::from_rendered (generator.gen_range (0.001f32.log2(), 1f32.log2()).exp2()),
    sustain: UserNumber::from_rendered (generator.gen_range (0.25f32.log2(), 3f32.log2()).exp2()),
    decay: UserNumber::from_rendered (generator.gen_range (0.25f32.log2(), 3f32.log2()).exp2()),
  }
}
pub fn random_signal <G: Rng, T: UserNumberType>(generator: &mut G, info: & SignalInfo)->Signal <T> {
  let mut effects = Vec::new() ;
  while generator.gen() {
    effects.push (random_signal_effect (generator, info));
  }
  Signal {
    initial_value: UserNumber::from_rendered (generator.gen_range (info.slider_range [0], info.slider_range [1])),
    constant: effects.len() == 0,
    effects: effects,
  }
}

pub fn random_signal_effect <G: Rng, T: UserNumberType>(generator: &mut G, info: & SignalInfo)->SignalEffect <T> {
  match generator.gen_range (0, 3) {
    0 => random_jump_effect (generator, info),
    1 => random_slide_effect (generator, info),
    2 => random_oscillation_effect (generator, info),
    _ => unreachable!(),
  }
}
pub fn random_jump_effect <G: Rng, T: UserNumberType>(generator: &mut G, info: & SignalInfo)->SignalEffect <T> {
  SignalEffect::Jump {
    time: random_time (generator),
    size: UserNumber::from_rendered (generator.gen_range (- info.difference_slider_range, info.difference_slider_range)),
  }
}
pub fn random_slide_effect <G: Rng, T: UserNumberType>(generator: &mut G, info: & SignalInfo)->SignalEffect <T> {
  SignalEffect::Slide {
    start: random_time (generator),
    duration: random_time (generator),
    size: UserNumber::from_rendered (generator.gen_range (- info.difference_slider_range, info.difference_slider_range)),
    smooth_start: generator.gen(),
    smooth_stop: generator.gen(),
  }
}
pub fn random_oscillation_effect <G: Rng, T: UserNumberType>(generator: &mut G, info: & SignalInfo)->SignalEffect <T> {
  SignalEffect::Oscillation {
    size: UserNumber::from_rendered (generator.gen_range (- info.difference_slider_range, info.difference_slider_range)),
    waveform: random_waveform (generator),
    frequency: UserNumber::from_rendered (generator.gen_range (1f32.log2(), 20f32.log2())),
  }
}


pub fn random_sound <G: Rng>(generator: &mut G)->SoundDefinition {
  let mut sound = SoundDefinition {
    waveform: random_waveform(generator),
    envelope: random_envelope (generator),
    ..Default::default()
  };
  struct Visitor <'a, G: 'a + Rng> (& 'a mut G);
  impl<'a, G: Rng> SignalVisitorMut for Visitor<'a, G> {
    fn visit_mut <T: UserNumberType> (&mut self, info: &SignalInfo, signal: & mut Signal <T>, _getter: Getter <State, Signal <T>>) {
      *signal = random_signal (self.0, info);
    }
  }
  
  {
    let mut visitor = Visitor (generator);
    for caller in sound.visit_mut_callers::<Visitor<G>>() {
      //let hack: Box <Fn (Visitor<G>, &mut SoundDefinition)> = caller;
      //(hack)(, &mut sound);
      (caller)(&mut visitor, &mut sound);
    }
  }
  
  for _attempt in 0..90 {
    let volume_range = sound.volume.range();
    if volume_range[1] > 1.0 || (volume_range[0] + volume_range[1] <= 0.0) {
      sound.volume = random_signal (generator, &SignalInfo::volume());
    }
    if sound.log_lowpass_filter_cutoff.range() [0] < sound.log_frequency.range() [1] {
      sound.log_lowpass_filter_cutoff = random_signal (generator, &SignalInfo::log_lowpass_filter_cutoff());
    }
    if sound.log_highpass_filter_cutoff.range() [1] > sound.log_frequency.range() [0] {
      sound.log_highpass_filter_cutoff = random_signal (generator, &SignalInfo::log_highpass_filter_cutoff());
    }
    if sound.log_bitcrush_frequency.range() [0] < sound.log_frequency.range() [1] {
      sound.log_bitcrush_frequency = random_signal (generator, &SignalInfo::log_bitcrush_frequency());
    }
  }
  
  sound
}
