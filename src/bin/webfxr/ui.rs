use std::rc::Rc;
use std::cell::RefCell;
use stdweb::{Value};

use super::*;


pub fn input_callback<T, F> (state: &Rc<RefCell<State>>, callback: F)->impl (Fn (T))
  where
    F: Fn(&mut State, T) {
  let state = state.clone();
  move |arg: T| {
    let mut sound_changed = false;
    {
      let mut guard = state.borrow_mut();
      let state = &mut*guard;
      (callback)(state, arg);
      if state.sound != state.undo_history [state.undo_position] {
        sound_changed = true;
        state.undo_history.split_off (state.undo_position + 1);
        state.undo_history.push_back (state.sound.clone());
        if state.undo_history.len() <= 1000 {
          state.undo_position += 1;
        } else {
          state.undo_history.pop_front();
        }
      }
    }
    if sound_changed {
      update_for_changed_sound (&state);
    }
  }
}

pub fn undo (state: &Rc<RefCell<State>>) {
  let mut sound_changed = false;
  {
    let mut guard = state.borrow_mut();
    let state = &mut*guard;
    if state.undo_position > 0 {
      state.undo_position -= 1;
      state.sound = state.undo_history [state.undo_position].clone();
      sound_changed = true;
    }
  }
  if sound_changed {
    update_for_changed_sound (&state);
  }
}
pub fn redo (state: &Rc<RefCell<State>>) {
  let mut sound_changed = false;
  {
    let mut guard = state.borrow_mut();
    let state = &mut*guard;
    if state.undo_position + 1 < state.undo_history.len() {
      state.undo_position += 1;
      state.sound = state.undo_history [state.undo_position].clone();
      sound_changed = true;
    }
  }
  if sound_changed {
    update_for_changed_sound (&state);
  }
}

pub fn input_callback_nullary<F> (state: &Rc<RefCell<State>>, callback: F)->impl (Fn ())
  where
    F: Fn(&mut State) {
  let hack = input_callback (state, move | state,()| (callback)(state));
  move || {
    (hack)(())
  }
}

pub fn input_callback_gotten<T, U, F> (state: &Rc<RefCell<State>>, getter: Getter <State, T>, callback: F)->impl (Fn (U))
  where
    F: Fn(&mut T, U) {
  let getter = getter.clone();
  input_callback (state, move | state, arg | (callback)(getter.get_mut (state), arg))
}

pub fn input_callback_gotten_nullary<T, F> (state: &Rc<RefCell<State>>, getter: Getter <State, T>, callback: F)->impl (Fn ())
  where
    F: Fn(&mut T) {
  let getter = getter.clone();
  input_callback_nullary (state, move | state | (callback)(getter.get_mut (state)))
}


pub struct Canvas {
  pub canvas: Value,
  pub context: Value,
}

/*pub enum SamplesCanvasKind {
  Signal,
  Rendered,
}*/


pub struct IllustrationCanvas {
  pub canvas: Canvas,
  //pub kind: SamplesCanvasKind,
  pub lines_drawn: usize,
  pub state: Rc<RefCell<State>>,
  pub getter: Getter <RenderingState, Illustration>,
}

impl Default for Canvas {
  fn default()->Canvas {
    let canvas = js!{ return ($(new_canvas ())); };
    let context = js!{ return @{&canvas}[0].getContext ("2d"); };
    Canvas {canvas, context}
  }
}

impl IllustrationCanvas {
  pub fn new (state: Rc<RefCell<State>>, getter: Getter <RenderingState, Illustration>)->IllustrationCanvas {
    IllustrationCanvas {
      canvas: Default::default(),
      lines_drawn: 0,
      state: state,
      getter: getter,
    }
  }
 pub fn draw_line (&self, illustration: & Illustration, index: usize) {
    let line = & illustration.lines [index];
    
    js!{
      var canvas = @{&self.canvas.canvas}[0];
      var context = @{&self.canvas.context};

      context.fillStyle = @{line.clipping} ? "rgb(255,0,0)" : "rgb(0,0,0)";
      
      context.fillRect (@{index as f64}, canvas.height*(1 -@{line.range [1]})-0.5, 1, canvas.height*@{line.range [1] - line.range [0]}+1.0);
    }
  }
  
  pub fn draw_next_line (&mut self, illustration: & Illustration) {
    self.draw_line (illustration, self.lines_drawn);
    self.lines_drawn += 1;
  }
  
  pub fn reset (&self) {
    js!{
      var canvas = @{&self.canvas.canvas}[0];
      var context = @{&self.canvas.context};

      context.clearRect (0, 0, canvas.width, canvas.height);
    }
  }
  
  pub fn update (&mut self, state: & State) {
    let illustration = self.getter.get (& state.rendering_state);
    //println!("{:?}", (self.lines_drawn, illustration.lines.len()));
    while self.lines_drawn <illustration.lines.len() {
      self.draw_next_line(illustration);
    }
  }
  
  pub fn redraw (&mut self, state: & State, playback_position: Option <f64>, constants: & RenderingStateConstants) {
    self.reset();
    self.update(state);
    
    if let Some(playback_position) = playback_position {
      let index = (playback_position*constants.sample_rate as f64/constants.samples_per_illustrated as f64).floor();
      js!{
        var canvas = @{&self.canvas.canvas}[0];
        var context = @{&self.canvas.context};

        context.fillStyle = "rgb(255,255,0)";
        context.fillRect (@{index as f64}, 0, 1, canvas.height);
      }
    }
  }

}
    
/*



pub fn display_samples <F: FnMut(f64)->f64> (sample_rate: f64, duration: f64, mut sampler: F)->Vec<f64> {
  let duration = min (duration, MAX_RENDER_LENGTH);
  let num_samples = (duration*sample_rate).ceil() as usize + 1;
  (0..num_samples).map (| sample | sampler (sample as f64/sample_rate)).collect()
}

pub fn canvas_of_samples (samples: & [f64], sample_rate: f64, canvas_height: f64, default_range: [f64; 2], target_duration: f64)->Value {
  let canvas = js!{ return ($(new_canvas ()));};
  draw_samples (canvas.clone(), samples, sample_rate, canvas_height, default_range, target_duration);
  canvas
}

pub fn draw_samples (canvas: Value, samples: & [f64], sample_rate: f64, canvas_height: f64, default_range: [f64; 2], target_duration: f64) {
  let min_sample = *samples.iter().min_by_key (| value | OrderedFloat (**value)).unwrap();
  let max_sample = *samples.iter().max_by_key (| value | OrderedFloat (**value)).unwrap();
  let default_range_size = default_range [1] - default_range [0];
  let min_displayed = min(min_sample, default_range [0]);
  let max_displayed = max(max_sample, default_range [1]);
  let draw_min = min_sample < default_range [0] - 0.0001*default_range_size;
  let draw_max = max_sample > default_range [1] + 0.0001*default_range_size;
  let range_displayed = max_displayed - min_displayed;
  let duration_displayed = samples.len() as f64/sample_rate;
  let draw_duration = duration_displayed > target_duration + 0.01;
  
  let display_height = | sample | (max_displayed - sample)/range_displayed*canvas_height;
  let display_x_time = | time | time*DISPLAY_SAMPLE_RATE;
  let display_x = | index | display_x_time((index as f64 + 0.5)/sample_rate);
  
  
  
  let context = js!{
    var canvas = @{& canvas}[0];
    canvas.width = @{duration_displayed*DISPLAY_SAMPLE_RATE};
    canvas.height = @{canvas_height};
    var context = canvas.getContext ("2d") ;
    return context;
  };
  
  js!{
    var canvas = @{& canvas}[0];
    var context =@{&context};
    context.strokeStyle = "rgb(128,0,0)";
    context.stroke();
    if (@{draw_min}) {
      context.beginPath();
      context.moveTo (0,@{display_height (default_range [0])});
      context.lineTo (canvas.width,@{display_height (default_range [0])});
      context.stroke();
    }
    if (@{draw_max}) {
      context.beginPath();
      context.moveTo (0,@{display_height (default_range [1])});
      context.lineTo (canvas.width,@{display_height (default_range [1])});
      context.stroke();
    }
    if (@{draw_duration}) {
      context.beginPath();
      context.moveTo (@{display_x_time(target_duration)},0);
      context.lineTo (@{display_x_time(target_duration)},canvas.height);
      context.stroke();
    }
    context.beginPath();
  }
    
  for (index, &sample) in samples.iter().enumerate() {
    js!{
      var context =@{&context};
      var first = @{display_x(index)};
      var second = @{display_height (sample)};
      if (@{index == 0}) {
        context.moveTo (first, second);
      } else {
        context.lineTo (first, second);
      }
    }
  }
    
  js!{
    var context =@{&context};
    context.strokeStyle = "rgb(0,0,0)";
    context.stroke();
  }
}*/

pub fn make_rendered_canvas (state: &Rc<RefCell<State>>, rendered_getter: Getter <RenderingState, RenderedSamples>, height: i32)->IllustrationCanvas {
  let mut result = IllustrationCanvas::new (state.clone(), rendered_getter.clone() + getter! (samples: RenderedSamples => samples.illustration));
  //let guard = state.borrow();
  //let rendered = rendered_getter.get (& guard.rendering_state);
  js!{
    var canvas = @{result.canvas.canvas.clone()};
    canvas[0].height =@{height};
    canvas[0].width =@{MAX_RENDER_LENGTH*DISPLAY_SAMPLE_RATE};
    on (canvas, "click", function() {@{{
      let state = state.clone() ;
      let getter = rendered_getter.clone();
      move || {
        let mut guard = state.borrow_mut() ;
        play (&mut guard, getter.clone());
      }
    }}();});
  }
  //rendered.redraw (None, & guard.rendering_state.constants);
  result.reset();
  result.update(& state.borrow());
  result
}
