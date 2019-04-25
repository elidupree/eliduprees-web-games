#![feature(never_type, nll)]
#![recursion_limit = "256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate array_ext;
extern crate nalgebra;
extern crate ordered_float;
extern crate rand;
extern crate rand_xoshiro;

use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use std::mem;
use std::ops::Bound;
use std::rc::Rc;
use stdweb::Value;
//use stdweb::unstable::TryInto;
pub use array_ext::Array;
pub use eliduprees_web_games::*;
use ordered_float::OrderedFloat;
use stdweb::web::{self, TypedArray};

#[macro_use]
mod misc;
#[macro_use]
mod data;
mod inputs;
mod randomization;
mod rendering;
mod ui;
mod generate_static_files;
pub use data::*;
pub use inputs::*;
pub use misc::*;
pub use randomization::*;
pub use rendering::*;
pub use ui::*;

type SoundId = u64;

#[derive(Clone)]
pub enum PlaybackTime {
  RunningSinceAudioTime(f64),
  WaitingAtOffset(f64),
}

impl PlaybackTime {
  fn current_offset(&self) -> f64 {
    match self {
      PlaybackTime::RunningSinceAudioTime(start) => audio_now() - start,
      PlaybackTime::WaitingAtOffset(offset) => *offset,
    }
  }
}

#[derive(Clone)]
pub struct Playback {
  time: PlaybackTime,
  samples_getter: DynamicGetter<RenderingState, RenderedSamples>,
}

pub struct State {
  pub sound: SoundDefinition,
  pub undo_history: VecDeque<SoundDefinition>,
  pub undo_position: usize,
  /*pub sounds: Vec <(SoundId, Option<SoundDefinition>)>,
  pub selected_sound: SoundId,
  pub undo_history: VecDeque <(SoundId, Option <SoundDefinition>)>,
  pub redo_stack: Vec<(SoundId, Option <SoundDefinition>)>,*/
  pub rendering_state: RenderingState,
  pub playback_state: Option<Playback>,
  pub loop_playback: bool,
  pub waveform_canvas: Canvas,
  pub effects_shown: HashSet<&'static str>,
  pub render_progress_functions: Vec<Box<dyn FnMut()>>,
}

fn update_for_changed_sound(state: &Rc<RefCell<State>>) {
  restart_rendering(state);
  redraw_app(state);
  play(
    &mut state.borrow_mut(),
    getter! (state: RenderingState => RenderedSamples {state.final_samples}),
  );
}

fn restart_rendering(state: &Rc<RefCell<State>>) {
  let mut guard = state.borrow_mut();
  let state = &mut *guard;
  state.rendering_state = RenderingState::new(&state.sound);
}

pub struct RedrawState {
  pub rows: u32,
  pub main_grid: Value,
  pub render_progress_functions: Vec<Box<dyn FnMut(&State)>>,
}

pub trait UIBuilder {
  #[inline(always)]
  fn css (&mut self, _css: & str) {}
  #[inline(always)]
  fn next_grid_row_class (&mut self, _classname: & str) {}
  #[inline(always)]
  fn last_n_grid_rows_class (&mut self, _classname: & str, _n: i32) {}
  #[inline(always)]
  fn add_event_listener <Event: ConcreteEvent, Callback: FnMut(Event)> (&mut self, _id: String, _listener: Callback) {}
  #[inline(always)]
  fn add_event_listener_erased <Callback: FnMut(Value)> (&mut self, _id: String, _event_type: &'static str, _listener: Callback) {}
  #[inline(always)]
  fn after_morphdom<Callback: FnOnce()> (&mut self, _callback: Callback) {}
  #[inline(always)]
  fn on_render_progress<Callback: FnMut()> (&mut self, _callback: Callback) {}
}

pub fn envelope_input<Builder: UIBuilder, G: 'static + GetterBase<From = State, To = UserTime>>(builder: &mut Builder, id: &str, name: &str, range: [f64; 2], getter: Getter<G>) -> Vec<Element; 2> {
  let (input, label) = numerical_input(
      builder,
      id, name, getter, range,

    );
  
  builder.next_grid_row_class(id);
    
  [
    html! {
      <div class=[id, "grid_row_label"]>{label}</div>
    },
    html! {
      <div class=[id, "grid_numerical"]>{input}</div>
    }
  ]
}

fn app<Builder: UIBuilder>(builder: &mut Builder) -> Element {
  with_state(|state| {
  let (loop_input, loop_label) = checkbox_input (
        builder,
        "loop",
        "Loop",
        getter! (state: State => bool{ state.loop_playback}),
      );
  let mut envelope_inputs = Vec::new();
  envelope_inputs.extend(envelope_input(builder, "attack", "Attack", [0.0, 1.0], getter! (state: State => UserTime {state.sound.envelope.attack})));
  envelope_inputs.extend(envelope_input(builder, "sustain", "Sustain", [0.0, 3.0], getter! (state: State => UserTime {state.sound.envelope.sustain})));
  envelope_inputs.extend(envelope_input(builder, "decay", "Decay", [0.0, 3.0], getter! (state: State => UserTime {state.sound.envelope.decay})));
  
  builder.last_n_grid_rows_class("envelope", 3);
  
  builder.next_grid_row_class("waveform");
  let (main_waveform_input, waveform_label) = waveform_input (builder, "waveform", "Waveform", getter! (state: State => Waveform {state.sound.waveform}));
  
  let mut signal_elements = Vec::with_capacity (16);

    struct Visitor<'a>(&'a mut Builder);
    impl<'a> SignalVisitor for Visitor<'a> {
      fn visit<Identity: SignalIdentity>(&mut self) {
        let specification: SignalEditorSpecification<Identity> = SignalEditorSpecification {
          builder: self.0,
          _marker: PhantomData,
        };
        signal_elements.extend (specification.render());
      }
    }

    visit_signals(&mut Visitor(&mut builder));

  builder.next_grid_row_class("clipping");
  builder.next_grid_row_class("sample_rate");
  
    let (clipping_input, clipping_label) = RadioInputSpecification {
        builder: builder,
        id: "clipping",
        name: "Clipping behavior",
        getter: getter! (state:State  => bool{state.sound.soft_clipping}).dynamic(),
        options: &[(false, "Hard clipping"), (true, "Soft clipping")],
      }
      .render();
      
    let label = assign_row(
      redraw.rows,
      js! { return @{& clipping_input}.children("label").first();},
    );
    js! {@{&label}.addClass("toplevel_input_label")}
    js! { @{grid_element}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{redraw.rows}+" / span 1")); }
    js! {@{grid_element}.append (@{label},@{clipping_input}.addClass("sound_radio_input"));}
    redraw.rows += 1;

    let (sample_rate_input, sample_rate_label) = RadioInputSpecification {
        builder: builder,
        id: "sample_rate",
        name: "Output sample rate",
        getter: getter! (state:State  => u32{ state.sound.output_sample_rate}).dynamic(),
        options: &[(44100, "44100"), (48000, "48000")],
      }
      .render();

  builder.add_event_listener("play_button", |_:ClickEvent| {
    play(getter! (state: RenderingState => RenderedSamples {state.final_samples}));
  });
  builder.add_event_listener("save_button", |_:ClickEvent| {
    with_state(|state| {
          if state.rendering_state.finished() {
            js! {
              var date = new Date ();
              var date_string = date.getFullYear () + "-" + (date.getMonth () + 1) + "-" + date.getDate () + "-" + date.getHours ()  + "-" + date.getMinutes ()  + "-" + date.getSeconds () ;
              var filename ="webfxr-sound-" + date_string + ".wav";
              var wav = audioBufferToWav(@{&state.rendering_state.final_samples.audio_buffer});
              var blob = new window.Blob([ new DataView(wav) ], { type: "audio/wav" });
              download (blob, filename, "audio/wav");
            }
          }
    });
  });
  builder.add_event_listener("undo_button", |_:ClickEvent| {
    undo();
  });
  builder.add_event_listener("redo_button", |_:ClickEvent| {
    redo();
  });
  builder.add_event_listener("randomize_button", |_:ClickEvent| {
    with_state_mut(|state| {
      state.sound = random_sound(&mut rand::thread_rng());
    });
  });
  builder.add_event_listener("mutate_everything_button", |_:ClickEvent| {
    with_state_mut(|state| {
      SoundMutator {
            generator: &mut rand::thread_rng(),
            duration: Default::default(),
            flop_chance: 0.0,
            tweak_chance: 1.0,
            tweak_size: 0.05,
          }
          .mutate_sound(&mut state.sound);
    });
  });
  builder.add_event_listener("mutate_onething_button", |_:ClickEvent| {
    with_state_mut(|state| {
      SoundMutator {
            generator: &mut rand::thread_rng(),
            duration: Default::default(),
            flop_chance: 0.1,
            tweak_chance: 0.1,
            tweak_size: 1.0,
          }
          .mutate_sound(&mut state.sound);
    });
  });
  let load_callback = || {
    let loading = js_unwrap!{$("#json_area").val()};
    if let Ok(sound) = serde_json::from_str(&loading) {
      with_state_mut(|state| state.sound = sound);
    }
  });
  builder.add_event_listener("json_area", |_:ClickEvent| {
    js!{
      $("#json_area").select();
    }
  });
  builder.add_event_listener("json_area", |_:InputEvent| {
    let loading = js_unwrap!{$("#json_area").val()};
    if let Ok(sound) = serde_json::from_str(&loading) {
      with_state_mut(|state| state.sound = sound);
    }
  });
  
  html! {
    <div class="app">
      <div class="left_column">
        {main_canvas}
        <input type="button" id="play_button" value="Play" />
        <div class="labeled_input">
          {loop_input}
          {loop_label}
        </div>
        <input type="button" id="save_button" value="Save" />
        <input type="button" id="undo_button" value="Undo (z)" />
        <input type="button" id="redo_button" value="Redo (shift-Z)" />
        <input type="button" id="randomize_button" value="Randomize" />
        <input type="button" id="mutate_everything_button" value="Randomize everything a little" />
        <input type="button" id="mutate_onething_button" value="Randomize a few things a lot" />
        <textarea id="json_area">
          text!(serde_json::to_string_pretty (&state.sound).unwrap())
        </textarea>
      </div>
      <div class="main_grid">
        {envelope_inputs}
        <div class=["envelope", "envelope_canvas"]>{envelope_canvas}</div>
        <div class=["envelope", "input_region"]></div>
        <div class=["waveform", "grid_row_label"]>{waveform_label}</div>
        <div class=["waveform", "waveform_input"]>{main_waveform_input}</div>
        <div class=["waveform", "waveform_canvas"]>{waveform_canvas}</div>
        <div class=["waveform", "input_region"]></div>
        {signal_elements}
        <div class=["clipping", "grid_row_label"]>{clipping_label}</div>
        <div class=["clipping", "grid_main_input"]>{clipping_input}</div>
        <div class=["clipping", "input_region"]></div>
        <div class=["sample_rate", "grid_row_label"]>{sample_rate_label}</div>
        <div class=["sample_rate", "grid_main_input"]>{sample_rate_input}</div>
        <div class=["sample_rate", "input_region"]></div>
      </div>
    </div>
  }
  
  });
}


#[derive (Default)]
struct ClientSideUIBuilder {
  pub after_morphdom_functions: Vec<Box<dyn FnOnce()>>,
  pub render_progress_functions: Vec<Box<dyn FnMut()>>,
  pub event_listeners: HashMap <(String, & 'static str), Box <dyn FnMut (Value)>>,
}

impl UIBuilder for ClientSideUIBuilder {
  fn add_event_listener <Event: ConcreteEvent, Callback: FnMut(Event)> (&mut self, id: String, listener: Callback) {
    self.add_event_listener_erased (id, Event::EVENT_TYPE, move | event | (listener)(event.try_into().unwrap()));
  }
  fn add_event_listener_erased <Callback: FnMut(Value)> (&mut self, id: String, event_type: &'static str, listener: Callback) {
    self.event_listeners.insert ((id, event_type), Box::new (listener));
  }
  fn after_morphdom<Callback: FnOnce()> (&mut self, callback: Callback) {
    self.after_morphdom_functions.push (Box::new (callback));
  }
  fn on_render_progress<Callback: FnMut()> (&mut self, callback: Callback) {
    self.render_progress_functions.push (Box::new (callback));
  }
}



fn redraw_app() {
  let mut builder = ClientSideUIBuilder::default();
  let app_element = app(&mut builder);
  
  js! {morphdom($("#app")[0], @{app_element.vnode()});}
  for f in builder.after_morphdom_functions { (f)(); }
  
  with_state_mut (move | state | {
    state.render_progress_functions = builder.render_progress_functions;
    state.event_listeners = builder.event_listeners;
  });
}



    state.waveform_canvas = Canvas::default();


    let sample_rate = 500.0;
    //let envelope_samples = display_samples (sample_rate, sound.duration(), | time | sound.envelope.sample (time));
    
    
    let mut main_canvas = make_rendered_canvas(
      state,
      getter! (state: RenderingState => RenderedSamples {state.final_samples}),
      100,
    );
    redraw
      .render_progress_functions
      .push(Box::new(move |state| main_canvas.update(state)));


    let envelope_canvas = Canvas::default();
    js! {
      var canvas =@{&envelope_canvas.canvas}[0];
      var context =@{&envelope_canvas.context};
      canvas.height = 90;
      context.beginPath();
      var horizontal = 0;
      context.moveTo (0, canvas.height);
      horizontal += @{sound.envelope.attack.rendered*DISPLAY_SAMPLE_RATE};
      context.lineTo (horizontal, 0);
      horizontal += @{sound.envelope.sustain.rendered*DISPLAY_SAMPLE_RATE};
      context.lineTo (horizontal, 0);
      horizontal += @{sound.envelope.decay.rendered*DISPLAY_SAMPLE_RATE};
      context.lineTo (horizontal, canvas.height);
      context.strokeStyle = "rgb(0,0,0)";
      context.stroke();
    }

    js! {@{grid_element}.append (
      @{& envelope_canvas.canvas}.parent()
      .css("grid-row", @{redraw.rows}+" / span 3")
    );}

    js! {@{grid_element}.append (@{assign_row(redraw.rows, js!{ return @{&guard.waveform_canvas.canvas}.parent()})});}
    redraw_waveform_canvas(&guard);


    //js! {window.before_render = Date.now();}
    //let rendered: TypedArray <f64> = sound.render (44100).as_slice().into();

    //js! {console.log("rendering took this many milliseconds: " + (Date.now() - window.before_render));}
}




fn redraw_waveform_canvas(state: &State) {
  //let sample_rate = 500.0;
  //let waveform_samples = display_samples (sample_rate, 3.0, | phase | state.sound.sample_waveform (time, phase));

  //draw_samples (state.waveform_canvas.clone(), &waveform_samples, sample_rate, 40.0, [-1.0, 1.0], 3.0);

  js! {
    var canvas =@{&state.waveform_canvas.canvas}[0];
    var context =@{&state.waveform_canvas.context};
    //canvas.width = 100;
    //canvas.height = 200;
    context.clearRect (0, 0, canvas.width, canvas.height);
  }

  let rendering = &state.rendering_state;
  let (start_time, samples) = match state.playback_state {
    None => (
      state.sound.envelope.attack.rendered,
      &rendering.final_samples,
    ),
    Some(ref playback) => (
      playback.time.current_offset(),
      playback.samples_getter.get(rendering),
    ),
  };

  let start_time = match rendering
    .cycle_starts
    .range((Bound::Unbounded, Bound::Included(OrderedFloat(start_time))))
    .rev()
    .next()
  {
    None => return,
    Some(time) => time.0,
  };

  let frequency = resample(
    &rendering.signals.get::<LogFrequency>().samples,
    start_time * rendering.constants.sample_rate as f64,
  )
  .exp2();
  let wavelength = 1.0 / frequency;
  let duration = wavelength * 3.0;
  let rendered_duration = samples.samples.len() as f64 / rendering.constants.sample_rate as f64;
  //eprintln!("{:?}", (rendered_duration, wavelength, start_time));
  if rendered_duration >= start_time + duration {
    js! {
      var canvas =@{&state.waveform_canvas.canvas}[0];
      var context =@{&state.waveform_canvas.context};

      context.beginPath();
    }
    let num_samples = 500;
    for index in 0..num_samples {
      let fraction = index as f64 / (num_samples - 1) as f64;
      let time = start_time + duration * fraction;
      let value = samples.resample(time, &rendering.constants);
      //eprintln!("{:?}", (time, value));
      js! {
        var canvas =@{&state.waveform_canvas.canvas}[0];
        var context =@{&state.waveform_canvas.context};
        var first =@{fraction}*canvas.width;
        var second =(0.5 - @{value}*0.5)*canvas.height;

        if (@{index == 0}) {
          context.moveTo (first, second);
        } else {
          context.lineTo (first, second);
          //console.log(first, second);
        }
      }
    }
    js! {
      var canvas =@{&state.waveform_canvas.canvas}[0];
      var context =@{&state.waveform_canvas.context};

      context.strokeStyle = "rgb(0,0,0)";
      context.stroke();
    }
  }
}

const SWITCH_PLAYBACK_DELAY: f64 = 0.15;

fn render_loop(state: Rc<RefCell<State>>) {
  {
    let mut guard = state.borrow_mut();
    let state = &mut *guard;
    let start = now();

    let already_finished = state.rendering_state.finished();

    while !state.rendering_state.finished() {
      {
        state.rendering_state.step(&state.sound);
      }
      let elapsed = now() - start;
      if elapsed > 0.005 {
        break;
      }
    }

    if state.rendering_state.finished() && !already_finished {
      println!(
        "Rendering took {} ms",
        ((now() - state.rendering_state.constants.started_rendering_at) * 1000.0).round()
      );
    }

    js! {
      $("#save_button").prop("disabled", @{!state.rendering_state.finished()});
    }

    if !already_finished {
      let mut functions = mem::replace(&mut state.render_progress_functions, Default::default());
      for function in &mut functions {
        (function)(state);
      }
      state.render_progress_functions = functions;
    }

    let mut stopped_waiting = false;

    let rendered_duration = state.rendering_state.final_samples.samples.len() as f64
      / state.rendering_state.constants.sample_rate as f64;
    if state.rendering_state.finished() || rendered_duration >= 0.02 {
      if let Some(ref mut playback) = state.playback_state {
        if let PlaybackTime::WaitingAtOffset(offset) = playback.time {
          let time_spent_rendering = now() - state.rendering_state.constants.started_rendering_at;
          let rendering_speed = rendered_duration / time_spent_rendering;
          let currently_available_playback_time = rendered_duration - offset;
          let conservative_rendering_speed = rendering_speed - 0.05;
          if state.rendering_state.finished() || conservative_rendering_speed >= 1.0 || {
            let expected_available_playback_time =
              currently_available_playback_time / (1.0 - conservative_rendering_speed);
            expected_available_playback_time > 1.2
          } {
            playback.time =
              PlaybackTime::RunningSinceAudioTime(audio_now() + SWITCH_PLAYBACK_DELAY - offset);
            stopped_waiting = true;
          }
        }
      }
    }

    if stopped_waiting || !already_finished {
      if let Some(ref mut playback) = state.playback_state {
        if let PlaybackTime::RunningSinceAudioTime(start) = playback.time {
          let now: f64 = audio_now();
          let offset = now - start;
          let transition_time = now + SWITCH_PLAYBACK_DELAY;
          let offset_then = transition_time - start;
          if offset_then > rendered_duration {
            if !state.rendering_state.finished() {
              playback.time = PlaybackTime::WaitingAtOffset(offset);
            }
          } else {
            js! {
              play_buffer (@{transition_time}, @{&playback.samples_getter.get(&state.rendering_state).audio_buffer},@{offset_then},@{rendered_duration - offset_then});
            }
          }
        }
      }
    }

    if let Some(playback) = state.playback_state.clone() {
      let offset = playback.time.current_offset();
      if offset > state.sound.rendering_duration() {
        if state.loop_playback {
          play(state, playback.samples_getter);
        } else {
          let samples = playback.samples_getter.get(&state.rendering_state);
          //samples.redraw (None, & state.rendering_state.constants);
          state.playback_state = None;
        }
      } else if let PlaybackTime::RunningSinceAudioTime(_) = playback.time {
        let samples = playback.samples_getter.get(&state.rendering_state);
        //samples.redraw (Some(offset), & state.rendering_state.constants);
      }
      redraw_waveform_canvas(state);
    }
  }

  web::window().request_animation_frame(move |_time| render_loop(state));
}

fn play<G: 'static + GetterBase<From = RenderingState, To = RenderedSamples>>(
  state: &mut State,
  getter: Getter<G>,
) {
  let samples = getter.get(&state.rendering_state);
  /*if let Some(ref playback) = state.playback_state {
    let old_samples = playback.samples_getter.get (&state.rendering_state);
    if old_samples.serial_number != samples.serial_number {
      old_samples.redraw (None, & state.rendering_state.constants);
    }
  }*/

  //let now: f64 = js!{return audio.currentTime;}.try_into().unwrap();
  state.playback_state = Some(Playback {
    time: PlaybackTime::WaitingAtOffset(0.0),
    samples_getter: getter.dynamic(),
  });
  /*js! {
    play_buffer (@{&samples.audio_buffer});
  }*/
}

#[cfg(target_os = "emscripten")]
fn main() {
  stdweb::initialize();

  let sound = SoundDefinition::default();
  let mut undo_history = VecDeque::new();
  undo_history.push_back(sound.clone());

  let state = Rc::new(RefCell::new(State {
    sound: sound,
    undo_history: undo_history,
    undo_position: 0,
    rendering_state: Default::default(),
    playback_state: None,
    loop_playback: false,
    waveform_canvas: Canvas::default(),
    effects_shown: HashSet::new(),
    render_progress_functions: Default::default(),
  }));

  js! { $(document.body).on ("keydown", function(event) {
    //if (event.ctrlKey || event.metaKey) {
      if (event.key === "z") {
        @{{ let state = state.clone(); move || undo (&state) }}();
        event.preventDefault();
      }
      if (event.key === "Z" || event.key === "y") {
        @{{ let state = state.clone(); move || redo (&state) }}();
        event.preventDefault();
      }
    //}
  });}

  update_for_changed_sound(&state);
  render_loop(state.clone());

  stdweb::event_loop();
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
  self::generate_static_files::generate_static_files();
}
