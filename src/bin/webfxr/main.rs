#![feature (never_type, nll)]
#![recursion_limit="256"]

extern crate eliduprees_web_games;

#[macro_use]
extern crate stdweb;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate derivative;
extern crate nalgebra;
extern crate rand;
extern crate ordered_float;
extern crate array_ext;

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{VecDeque, HashSet};
use std::marker::PhantomData;
use std::mem;
use std::ops::Bound;
use stdweb::Value;
//use stdweb::unstable::TryInto;
use stdweb::web::{self, TypedArray};
use ordered_float::OrderedFloat;
pub use array_ext::Array;
pub use eliduprees_web_games::*;


#[macro_use]
mod misc;
#[macro_use]
mod data;
mod rendering;
mod ui;
mod inputs;
mod randomization;
pub use misc::*;
pub use data::*;
pub use rendering::*;
pub use ui::*;
pub use inputs::*;
pub use randomization::*;


#[derive (Clone)]
pub enum PlaybackTime {
  RunningSinceAudioTime (f64),
  WaitingAtOffset (f64),
}

impl PlaybackTime {
  fn current_offset (&self)->f64 {match self {
    PlaybackTime::RunningSinceAudioTime (start) => audio_now() - start,
    PlaybackTime::WaitingAtOffset (offset) => *offset,
  }}
}

#[derive (Clone)]
pub struct Playback {
  time: PlaybackTime,
  samples_getter: DynamicGetter <RenderingState, RenderedSamples>,
}

pub struct State {
  pub sound: SoundDefinition,
  pub undo_history: VecDeque <SoundDefinition>,
  pub undo_position: usize,
  pub rendering_state: RenderingState,
  pub playback_state: Option <Playback>,
  pub loop_playback: bool,
  pub waveform_canvas: Canvas,
  pub effects_shown: HashSet <&'static str>,
  pub render_progress_functions: Vec<Box<dyn FnMut(& State)>>,
}



fn update_for_changed_sound (state: & Rc<RefCell<State>>) {
  restart_rendering (state);
  redraw_app (state);
  play (&mut state.borrow_mut(), getter! (state: RenderingState => RenderedSamples {state.final_samples}));
}

fn restart_rendering (state: & Rc<RefCell<State>>) {
  let mut guard = state.borrow_mut();
  let state = &mut*guard;
  state.rendering_state = RenderingState::new (& state.sound);
}



pub struct RedrawState {
  pub rows: u32,
  pub main_grid: Value,
  pub render_progress_functions: Vec<Box<dyn FnMut(& State)>>,
}

fn redraw_app(state: & Rc<RefCell<State>>) {
  let mut redraw;
  {
    let mut guard = state.borrow_mut();
    let state = &mut*guard;
    
    state.waveform_canvas = Canvas::default();
  }
  {
  let guard = state.borrow();
  let sound = & guard.sound;
  
  pub fn assign_row (rows: u32, element: Value)->Value {
    js!{@{&element}.css("grid-row", @{rows}+" / span 1")};
    element
  }
  
  let sample_rate = 500.0;
  //let envelope_samples = display_samples (sample_rate, sound.duration(), | time | sound.envelope.sample (time));
      
  js!{clear_callbacks();}  
  let app_element = js!{ return ($("<div>", {id: "app"}));};
  let app_element = & app_element;
  let left_column = js!{
    return ($("<div>", {id: "left_column", class: "left_column"}).appendTo (@{app_element}));
  };
  let left_column = & left_column;
  let grid_element = js!{
    return ($("<div>", {id: "main_grid", class: "main_grid"}).appendTo (@{app_element}));
  };
  let grid_element = &grid_element;
  redraw = RedrawState {rows: 1, main_grid: grid_element.clone(), render_progress_functions: Vec::new()};
  

  let mut main_canvas = make_rendered_canvas (state, getter! (state: RenderingState => RenderedSamples {state.final_samples}), 100);
  js!{@{left_column}.append (@{& main_canvas.canvas.canvas}.parent());}
  redraw.render_progress_functions.push (Box::new (move | state | main_canvas.update(state)));
  //redraw.rows += 1;
      
  let play_button = assign_row (redraw.rows, button_input ("Play",
    { let state = state.clone(); move || {
      play (&mut state.borrow_mut(), getter! (state: RenderingState => RenderedSamples {state.final_samples}));
    }}
  ));
  js!{@{left_column}.append (@{play_button});}
  
  let loop_button = assign_row (redraw.rows, checkbox_input (state, "loop", "Loop", getter! (state: State => bool{ state.loop_playback})));
  js!{@{left_column}.append (@{loop_button});}
  
  let undo_button = assign_row (redraw.rows, button_input ("Undo (z)",
    { let state = state.clone(); move || undo (&state) }
  ));
  js!{@{left_column}.append (@{undo_button});}
  
  let redo_button = assign_row (redraw.rows, button_input ("Redo (shift-Z)",
    { let state = state.clone(); move || redo (&state) }
  ));
  js!{@{left_column}.append (@{redo_button});}
      
  let randomize_button = assign_row (redraw.rows, button_input ("Randomize",
    input_callback_nullary (state, move | state | {
      state.sound = random_sound (&mut rand::thread_rng());
    })
  ));
  js!{@{left_column}.append (@{randomize_button});}
  let randomize2_button = assign_row (redraw.rows, button_input ("Randomize everything a little",
    input_callback_nullary (state, move | state | {
      SoundMutator {
        generator: &mut rand::thread_rng(),
        duration: Default::default(),
        flop_chance: 0.0,
        tweak_chance: 1.0,
        tweak_size: 0.05,
      }.mutate_sound (&mut state.sound);
    })
  ));
  js!{@{left_column}.append (@{randomize2_button});}
  let randomize3_button = assign_row (redraw.rows, button_input ("Randomize a few things a lot",
    input_callback_nullary (state, move | state | {
      SoundMutator {
        generator: &mut rand::thread_rng(),
        duration: Default::default(),
        flop_chance: 0.05,
        tweak_chance: 0.05,
        tweak_size: 1.0,
      }.mutate_sound (&mut state.sound);
    })
  ));
  js!{@{left_column}.append (@{randomize3_button});}
  let load_callback = input_callback (state, | state, value: String | {
    if let Ok (sound) = serde_json::from_str (& value) {
      state.sound = sound;
    }
  });
  js!{
    on (on (
      $("<textarea>").text (@{serde_json::to_string_pretty (sound).unwrap()}).appendTo (@{left_column}),
      "click",
      function (event) {event.target.select() ;}
    ),
      "input",
      function(event) {@{load_callback} (event.target.value);}
    );}
  

  
  macro_rules! add_envelope_input {
  ($variable: ident, $name: expr, $range: expr) => {
    let input = assign_row(redraw.rows, numerical_input (
      state,
      stringify! ($variable),
      $name, 
      getter! (state: State => UserTime {state.sound.envelope.$variable}),
      $range
    ));
    
    let label = assign_row(redraw.rows, js!{ return @{&input}.children("label");});
    js!{@{&label}.append(":").addClass("toplevel_input_label")}
    js!{@{grid_element}.append (@{label},@{input});}
    redraw.rows += 1;
    }
  }
  
  let envelope_canvas = Canvas::default();
  js!{
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

  js!{@{grid_element}.append (
    @{& envelope_canvas.canvas}.parent()
    .css("grid-row", @{redraw.rows}+" / span 3")
  );}
  js!{ @{grid_element}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{redraw.rows}+" / span 3")); }
  add_envelope_input!(attack, "Attack", [0.0, 1.0]);
  add_envelope_input!(sustain, "Sustain", [0.0, 3.0]);
  add_envelope_input!(decay, "Decay", [0.0, 3.0]);
  
  
  
  let waveform_start = redraw.rows;
  let waveform_input = assign_row (redraw.rows, waveform_input (state, "waveform", "Waveform", getter! (state:State => Waveform{state.sound.waveform})));
  let label = assign_row(redraw.rows, js!{ return @{&waveform_input}.children("label").first();});
  js!{@{&label}.addClass("toplevel_input_label")}
  
  js!{@{grid_element}.append (@{assign_row(redraw.rows, js!{ return @{&guard.waveform_canvas.canvas}.parent()})});}
  redraw_waveform_canvas (& guard);
  js!{@{grid_element}.append (@{label},@{waveform_input}.addClass("sound_radio_input"));}
  redraw.rows += 1;
  
  js!{ @{grid_element}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{waveform_start}+" / "+@{redraw.rows})); }
  
  
  
  struct Visitor <'a> (& 'a Rc<RefCell<State>>, & 'a mut RedrawState);
  impl<'a> SignalVisitor for Visitor<'a> {
    fn visit <Identity: SignalIdentity> (&mut self) {
      let specification: SignalEditorSpecification<Identity> = SignalEditorSpecification {
        state: self.0,
        redraw: self.1,
        _marker: PhantomData,
      };
      specification.render();
    }
  }
  
  visit_signals (&mut Visitor (state, &mut redraw));
  
  let clipping_input = assign_row (redraw.rows, RadioInputSpecification {
    state: state, id: "clipping", name: "Clipping behavior", getter: getter! (state:State  => bool{state.sound.soft_clipping}).dynamic(),
    options: &[
      (false, "Hard clipping"),
      (true, "Soft clipping"),
    ],  
  }.render());
  let label = assign_row(redraw.rows, js!{ return @{& clipping_input}.children("label").first();});
  js!{@{&label}.addClass("toplevel_input_label")}
  js!{ @{grid_element}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{redraw.rows}+" / span 1")); }
  js!{@{grid_element}.append (@{label},@{clipping_input}.addClass("sound_radio_input"));}
  redraw.rows += 1;
  
  
  let sample_rate_input = assign_row (redraw.rows, RadioInputSpecification {
    state: state, id: "sample_rate", name: "Output sample rate", getter: getter! (state:State  => u32{ state.sound.output_sample_rate}).dynamic(),
    options: &[
      (44100, "44100"),
      (48000, "48000"),
    ],  
  }.render());
  let label = assign_row(redraw.rows, js!{ return @{& sample_rate_input}.children("label").first();});
  js!{@{&label}.addClass("toplevel_input_label")}
  js!{ @{grid_element}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{redraw.rows}+" / span 1")); }
  js!{@{grid_element}.append (@{label},@{sample_rate_input}.addClass("sound_radio_input"));}
  redraw.rows += 1;

  
  //js! {window.before_render = Date.now();}
  //let rendered: TypedArray <f64> = sound.render (44100).as_slice().into();
  
  //js! {console.log("rendering took this many milliseconds: " + (Date.now() - window.before_render));}
  
  js!{morphdom($("#app")[0], @{app_element}[0]);} 
  
  // hack – suppress warning from incrementing rows unnecessarily at the end
  //#[allow (unused_variables)] let whatever = redraw.rows;
  
  
  
  }
  {
    let mut guard = state.borrow_mut();
    let state = &mut*guard;
    
    state.render_progress_functions = redraw.render_progress_functions;
  }
}


fn redraw_waveform_canvas (state: & State) {
  //let sample_rate = 500.0;
  //let waveform_samples = display_samples (sample_rate, 3.0, | phase | state.sound.sample_waveform (time, phase));
  
  //draw_samples (state.waveform_canvas.clone(), &waveform_samples, sample_rate, 40.0, [-1.0, 1.0], 3.0);
  
  js!{
    var canvas =@{&state.waveform_canvas.canvas}[0];
    var context =@{&state.waveform_canvas.context};
    //canvas.width = 100;
    //canvas.height = 200;
    context.clearRect (0, 0, canvas.width, canvas.height);
  }
  
  let rendering = & state.rendering_state;
  let (start_time, samples) = match state.playback_state {
    None => (state.sound.envelope.attack.rendered, & rendering.final_samples),
    Some (ref playback) => (playback.time.current_offset(), playback.samples_getter.get (rendering)),
  };
  
  let start_time = match rendering.cycle_starts.range ((Bound::Unbounded, Bound::Included (OrderedFloat (start_time)))).rev().next() {
    None => return,
    Some (time) => time.0,
  };
  
  let frequency = resample (& rendering.signals.get::<LogFrequency>().samples, start_time*rendering.constants.sample_rate as f64).exp2();
  let wavelength = 1.0/frequency;
  let duration = wavelength*3.0;
  let rendered_duration = samples.samples.len() as f64/rendering.constants.sample_rate as f64;
  //eprintln!("{:?}", (rendered_duration, wavelength, start_time));
  if rendered_duration >= start_time + duration {
    
    js!{
      var canvas =@{&state.waveform_canvas.canvas}[0];
      var context =@{&state.waveform_canvas.context};
      
      context.beginPath();
    }
    let num_samples = 500;
    for index in 0..num_samples {
      let fraction = index as f64/(num_samples-1) as f64;
      let time = start_time + duration*fraction;
      let value = samples.resample (time, & rendering.constants);
      //eprintln!("{:?}", (time, value));
      js!{
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
    js!{
      var canvas =@{&state.waveform_canvas.canvas}[0];
      var context =@{&state.waveform_canvas.context};
      
      context.strokeStyle = "rgb(0,0,0)";
      context.stroke();
    }
  }
}

const SWITCH_PLAYBACK_DELAY: f64 = 0.15;

fn render_loop (state: Rc<RefCell<State>>) {
  {
    let mut guard = state.borrow_mut();
    let state = &mut*guard;
    let start = now();
    
    let already_finished = state.rendering_state.finished();
    
    while !state.rendering_state.finished() {
      { state.rendering_state.step(& state.sound); }
      let elapsed = now() - start;
      if elapsed > 0.005 {
        break;
      }
    }
    
    if state.rendering_state.finished() && !already_finished {
      println!("Rendering took {} ms", ((now() - state.rendering_state.constants.started_rendering_at)*1000.0).round());
    }
    
    if !already_finished {
      let mut functions = mem::replace (&mut state.render_progress_functions, Default::default());
      for function in &mut functions {(function)(state);}
      state.render_progress_functions = functions;
    }
    
    let mut stopped_waiting = false;
    
    let rendered_duration = state.rendering_state.final_samples.samples.len() as f64/state.rendering_state.constants.sample_rate as f64;
    if state.rendering_state.finished() || rendered_duration >= 0.02 {if let Some(ref mut playback) = state.playback_state {if let PlaybackTime::WaitingAtOffset (offset) = playback.time {
      let time_spent_rendering = now() - state.rendering_state.constants.started_rendering_at;
      let rendering_speed = rendered_duration/time_spent_rendering;
      let currently_available_playback_time = rendered_duration - offset;
      let conservative_rendering_speed = rendering_speed - 0.05;
      if state.rendering_state.finished() || conservative_rendering_speed >= 1.0 || {
        let expected_available_playback_time = currently_available_playback_time/(1.0 - conservative_rendering_speed);
        expected_available_playback_time > 1.2} {
        playback.time = PlaybackTime::RunningSinceAudioTime (audio_now() + SWITCH_PLAYBACK_DELAY - offset);
        stopped_waiting = true;
      }
    }}}
    
    if stopped_waiting || !already_finished {
      if let Some(ref mut playback) = state.playback_state {if let PlaybackTime::RunningSinceAudioTime (start) = playback.time {
        let now: f64 = audio_now();
        let offset = now - start;
        let transition_time = now + SWITCH_PLAYBACK_DELAY;
        let offset_then = transition_time - start;
        if offset_then > rendered_duration {
          if !state.rendering_state.finished() {
            playback.time = PlaybackTime::WaitingAtOffset (offset);
          }
        }
        else {
          js! {
            play_buffer (@{transition_time}, @{&playback.samples_getter.get(&state.rendering_state).audio_buffer},@{offset_then},@{rendered_duration - offset_then});
          }
        }
      }}
    }
    
    if let Some(playback) = state.playback_state.clone() {
      let offset = playback.time.current_offset();
      if offset > state.sound.rendering_duration() {
        if state.loop_playback {
          play (state, playback.samples_getter);
        } else {
          let samples = playback.samples_getter.get (&state.rendering_state);
          //samples.redraw (None, & state.rendering_state.constants);
          state.playback_state = None;
        }
      } else if let PlaybackTime::RunningSinceAudioTime (_) = playback.time {
        let samples = playback.samples_getter.get (&state.rendering_state);
        //samples.redraw (Some(offset), & state.rendering_state.constants);
      }
      redraw_waveform_canvas (state);
    }
  }
  
  web::window().request_animation_frame (move | _time | render_loop (state));
}

fn play<G:'static + GetterBase<From=RenderingState, To=RenderedSamples>> (state: &mut State, getter: Getter <G>) {
  let samples = getter.get (&state.rendering_state);
  /*if let Some(ref playback) = state.playback_state {
    let old_samples = playback.samples_getter.get (&state.rendering_state);
    if old_samples.serial_number != samples.serial_number {
      old_samples.redraw (None, & state.rendering_state.constants);
    }
  }*/

  //let now: f64 = js!{return audio.currentTime;}.try_into().unwrap();
  state.playback_state = Some(Playback {
    time: PlaybackTime::WaitingAtOffset (0.0),
    samples_getter: getter.dynamic(),
  });
  /*js! {
    play_buffer (@{&samples.audio_buffer});
  }*/ 
}


//#[cfg (target_os = "emscripten")]
fn main() {
  stdweb::initialize();
  
  let sound = SoundDefinition::default();
  let mut undo_history = VecDeque::new();
  undo_history.push_back (sound.clone()) ;
  
  let state = Rc::new (RefCell::new (State {
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
  
  js!{ $(document.body).on ("keydown", function(event) {
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
  render_loop (state.clone());
    
  stdweb::event_loop();
}


/*#[cfg (not(target_os = "emscripten"))]
fn main() {
  println!("There's not currently a way to compile this natively");
}*/
