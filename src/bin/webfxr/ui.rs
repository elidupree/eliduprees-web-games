use std::rc::Rc;
use std::cell::RefCell;
use stdweb::unstable::{TryInto, TryFrom};
use stdweb::{JsSerialize, Value};
use ordered_float::OrderedFloat;

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


pub fn button_input<F: 'static + Fn()> (name: & str, callback: F)->Value {
  let result: Value = js!{
    return on ($("<input>", {
      type: "button",
      value: @{name}
    }), "click", function() {@{callback}();});
  };
  result
}

pub fn checkbox_input (state: &Rc<RefCell<State>>, id: & str, name: & str, getter: Getter <State, bool>)->Value {
  let current_value = getter.get (&state.borrow()).clone();
  let callback = input_callback_gotten (state, getter, | target, value: bool | *target = value);
  let result: Value = js!{
    return $("<div>", {class: "labeled_input checkbox"}).append (
      on ($("<input>", {type: "checkbox", id: @{id}, checked:@{current_value}}), "click", function(event) {@{callback}($(event.target).prop ("checked"));}),
      $("<label>", {"for": @{id}, text: @{name}})
    );
  };
  result
}

pub fn menu_input <T: 'static + Eq + Clone> (state: &Rc<RefCell<State>>, getter: Getter <State, T>, options: & [(T, &str)])->Value {
  let current_value = getter.get (&state.borrow()).clone();
  let menu = js!{
    return $("<select>");
  };
  let mut values = Vec::with_capacity (options.len());
  for & (ref value, name) in options.iter() {
    values.push (value.clone());
    js!{@{& menu}.append ($("<option>", {selected:@{*value == current_value}}).text(@{name}));}
  }
  js!{ on (@{& menu}, "change", function(event) {
    @{input_callback_gotten (state, getter, move | target, index: i32 | {
      if let Some(value) = values.get (index as usize) {
        *target = value.clone();
      }
    })}(event.target.selectedIndex)
  })};
  menu
}

pub fn waveform_input (state: &Rc<RefCell<State>>, id: & str, name: & str, getter: Getter <State, Waveform>)->Value {
  RadioInputSpecification {
    state: state, id: id, name: name,
    options: &[
      (Waveform::Sine, "Sine"),
      (Waveform::Square, "Square"),
      (Waveform::Triangle, "Triangle"),
      (Waveform::Sawtooth, "Sawtooth"),
      (Waveform::WhiteNoise, "White noise"),
    ],
    getter: getter,
  }.render()
  /*let result = js!{return $("<div>", {class: "labeled_input radio"}).append (
    $("<label>", {text:@{name} + ": "}),
    @{menu_input (state, getter, &[
      (Waveform::Sine, "Sine"),
      (Waveform::Square, "Square"),
      (Waveform::Triangle, "Triangle"),
      (Waveform::Sawtooth, "Sawtooth"),
      (Waveform::WhiteNoise, "White noise"),
    ])}
  );};
  result*/
}



//fn round_step (input: f64, step: f64)->f64 {(input*step).round()/step}

pub struct RadioInputSpecification <'a, T: 'a> {
  pub state: & 'a Rc<RefCell<State>>,
  pub id: & 'a str,
  pub name: & 'a str,
  pub options: & 'a [(T, & 'a str)],
  pub getter: Getter <State, T>,
}

impl <'a, T: Clone + Eq + 'static> RadioInputSpecification <'a, T>
  where
    T: JsSerialize,
    T: TryFrom<Value>,
    Value: TryInto<T>,
    <Value as TryInto<T>>::Error: ::std::fmt::Debug {
  pub fn render (self)->Value {
    let current_value = self.getter.get (& self.state.borrow()).clone();
    let result = js!{return $("<div>", {class: "labeled_input radio"}).append (
      $("<label>", {text:@{self.name} + ":"})
    );};
    
    let update = js!{
      return function (value) {@{input_callback_gotten (self.state, self.getter, | target, value: T | *target = value)}(value)}
    };
    
    for &(ref value, name) in self.options.iter() {
      let input = js!{ return on ($("<input>", {type: "button", id: @{self.id}+"_radios_" + @{&value}, value: @{name}}), "click", function() {@{& update} (@{&value})});};
      if *value == current_value {
        js!{ @{&input}.addClass("down"); }
      }
      js!{ @{&result}.append (@{input}); }
    }
    
    result
  }
}


pub fn numerical_input <T: UserNumberType> (state: &Rc<RefCell<State>>, id: & str, name: & str, getter: Getter <State, UserNumber <T>>, slider_range: [f64; 2])->Value {
  let current_value = getter.get (&state.borrow()).clone();
  NumericalInputSpecification {
    state: state, id: id, name: name,
    slider_range: slider_range,
    slider_step: 0.0,
    current_value: current_value,
    input_callback: input_callback_gotten (state, getter, | target, value: UserNumber <T> | *target = value),
  }.render()
}

pub struct NumericalInputSpecification <'a, T: UserNumberType, F> {
  pub state: & 'a Rc<RefCell<State>>,
  pub id: & 'a str,
  pub name: & 'a str,
  pub slider_range: [f64; 2],
  pub slider_step: f64,
  pub current_value: UserNumber <T>,
  pub input_callback: F,
}

impl <'a, F: 'static + Fn (UserNumber <T>), T: UserNumberType> NumericalInputSpecification<'a, T, F> {
  pub fn render (self)->Value {
    let value_type = T::currently_used (&self.state.borrow());
    let displayed_value = if value_type == self.current_value.value_type {self.current_value.source.clone()} else {value_type.approximate_from_rendered (self.current_value.rendered)};
    let slider_step = if self.slider_step != 0.0 { self.slider_step } else {(self.slider_range [1] - self.slider_range [0])/1000.0};
    let input_callback = self.input_callback;
    let update_callback = { let value_type = value_type.clone(); move | value: String |{
          if let Some(value) = UserNumber::new (value_type.clone(), value) {
            (input_callback)(value);
          }
        }};
    let to_rendered_callback = { let value_type = value_type.clone(); move | value: String |{
          if let Some(value) = UserNumber::new (value_type.clone(), value) {
            value.rendered
          }
          else {std::f64::NAN}
        }};
    let update = js!{return _.debounce(function(value) {
        var success = @{update_callback} (value);
        if (!success) {
          // TODO display some sort of error message
        }
      }, 200);};
    let range_input = js!{return $("<input>", {type: "range", id: @{self.id}+"_numerical_range", value:@{self.current_value.rendered}, min:@{self.slider_range [0]}, max:@{self.slider_range [1]}, step:@{slider_step} });};
    let number_input = js!{return $("<input>", {type: "number", id: @{self.id}+"_numerical_number", value:@{displayed_value}});};
    
    let range_overrides = js!{return function (parent) {
        var value = parent.children ("input[type=range]")[0].valueAsNumber;
        var source = @{{let value_type = value_type.clone(); move | value: f64 | value_type.approximate_from_rendered (value)}} (value);
        // immediately update the number input with the range input, even though the actual data editing is debounced.
        parent.children ("input[type=number]").val(source);
        @{&update}(source);
      }
;};
    let number_overrides = js!{return function (parent) {
        @{&update}(parent.children ("input[type=number]").val());
      }
;};
    
    
    let result: Value = js!{
      var result = $("<div>", {id: @{self.id}, class: "labeled_input numeric"}).append (
        on (@{&number_input}, "input", function (event) {@{&number_overrides} ($("#"+@{self.id}))}),
        on (@{&range_input}, "input", function (event) {@{&range_overrides} ($("#"+@{self.id}))}),
        $("<label>", {"for": @{self.id}+"_numerical_number", text:@{
          format! ("{} ({})", self.name, value_type.unit_name())
        }})
      );
      
      @{&range_input}.val(@{self.current_value.rendered});
      
      return result;
    };
    js! {
      on (@{&result}, "wheel", function (event) {
        if (window.webfxr_scrolling) {return;}
        var parent = $("#"+@{self.id});
        var number_input = parent.children ("input[type=number]");
        var value = @{to_rendered_callback} (number_input.val());
        var range_input = parent.children ("input[type=range]");
        if (isNaN (value)) {var value = range_input[0].valueAsNumber;}
        //console.log (event.originalEvent.deltaY);
        var increment = ((-event.originalEvent.deltaY) || event.originalEvent.deltaX || 0)*@{slider_step*0.5};
        if (@{slider_step == 1.0}) {
          if (increment > 0) {
            value = Math.floor(value) + 1;
          }
          if (increment < 0) {
            value = Math.ceil(value) - 1;
          }
        }
        else {value += increment;}
        var source = @{{let value_type = value_type.clone(); move | value: f64 | value_type.approximate_from_rendered (value)}} (value);
        range_input.val (value);
        number_input.val(source);
        @{& number_overrides}(parent);
        event.preventDefault();
        event.stopPropagation();
      });
    };
    result
  }
}


pub struct SignalEditorSpecification <'a, Identity: SignalIdentity> {
  pub state: & 'a Rc<RefCell<State>>,
  pub rows: &'a mut u32,
  pub main_grid: & 'a Value,
  pub _marker: PhantomData <Identity>,
}

impl <'a, Identity: SignalIdentity> SignalEditorSpecification <'a, Identity> {
  pub fn assign_row (&self, element: Value)->Value {
    js!{@{&element}.css("grid-row", @{*self.rows}+" / span 1")};
    element
  }

  pub fn numeric_input <U: UserNumberType> (&self, id: & str, name: & str, slider_range: [f64; 2], slider_step: f64, getter: Getter <State, UserNumber <U>>)->Value {
    let current_value = getter.get (&self.state.borrow()).clone();
    self.assign_row(NumericalInputSpecification {
      state: self.state,
      id: id,
      name: name, 
      slider_range: slider_range,
      slider_step: slider_step,
      current_value: current_value,
      input_callback: input_callback (self.state, move | state, value: UserNumber <U> | *getter.get_mut (state) = value),
    }.render())
  }

  pub fn time_input (&self, id: & str, name: & str, getter: Getter <State, UserTime>)->Value {
    self.numeric_input (id, name, [0.0, 3.0], 0.0, getter)
  }
  
  pub fn value_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <Identity::NumberType>>)->Value {
    let info = Identity::info();
    self.numeric_input (id, name, info.slider_range, info.slider_step, getter)
  }
  
  pub fn difference_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <<Identity::NumberType as UserNumberType>::DifferenceType>>)->Value {
    let info = Identity::info();
    self.numeric_input (id, name, [-info.difference_slider_range, info.difference_slider_range], 0.0, getter)
  }
  
  pub fn frequency_input (&self, id: & str, name: & str, getter: Getter <State, UserFrequency>)->Value {
    self.numeric_input (id, name, [1.0f64.log2(), 20f64.log2()], 0.0, getter)
  }
  
  pub fn checkbox_input (&self, id: & str, name: & str, getter: Getter <State, bool>)->Value {
    self.assign_row(checkbox_input (self.state, id, name, getter))
  }
  pub fn waveform_input (&self, id: & str, name: & str, getter: Getter <State, Waveform>)->Value {
    self.assign_row(waveform_input (self.state, id, name, getter))
  }


  pub fn render (self) {
    
      let guard = self.state.borrow();
      let sound = & guard.sound;
      let signals_getter = Identity::definition_getter();
      let state_getter = getter! (state => state.sound.signals) + signals_getter.clone();
      let info = Identity::info();
    let signal = getter.get (& guard.sound.signals);
    let first_row = *self.rows;
      
  let container = self.main_grid;
  
  //js!{@{& container}.append (@{info.name} + ": ");}
  
  let initial_value_input = self.value_input (
    & format! ("{}_initial", & info.id),
    info.name,
    state_getter.clone() + getter! (signal => signal.initial_value)
  );
  
  js!{@{& container}.append (@{&initial_value_input})}
  //let input_height = js!{ return @{&initial_value_input}.outerHeight()};
  let mut label = self.assign_row(js!{ return @{initial_value_input}.children("label");});
  if info.can_disable {
    js!{@{label}.remove();}
    let toggle = self.checkbox_input (
      & format! ("{}_enabled", & info.id),
      info.name,
      state_getter.clone() + getter! (signal => signal.enabled)
    );
    js!{@{&toggle}.appendTo(@{& container}).addClass("signal_toggle")}
    label = self.assign_row(js!{ return @{toggle}.children("label");});
  }
  js!{@{label}.append(":").appendTo(@{& container}).addClass("toplevel_input_label")}
  {
    let info = info.clone();
    let duration = sound.duration();
    let getter = state_getter.clone();
    let buttons = self.assign_row(js!{
      var menu = $("<select>", {class: "add_effect_buttons"}).append (
        $("<option>", {selected: true}).text("Add effect..."),
        $("<option>").text(@{info.name}+" jump"),
        $("<option>").text(@{info.name}+" slide"),
        $("<option>").text(@{info.name}+" oscillation")
      );
      on(menu, "change", function(event) {
        @{input_callback (self.state, move | state, index: i32 | {
          state.effects_shown.insert (info.id);
          let signal = getter.get_mut (state);
          match index {
            1 => signal.effects.push (random_jump_effect (&mut rand::thread_rng(), duration, &info)),
            2 => signal.effects.push (random_slide_effect (&mut rand::thread_rng(), duration, &info)),
            3 => signal.effects.push (random_oscillation_effect (&mut rand::thread_rng(), duration, &info)),
            _ => return,
          }
        })}(event.target.selectedIndex)
      });
      return menu;
    });
    
    js!{ @{& container}.append (@{buttons}); }
  }
  
  let rendered = Identity::rendered_getter() (& guard.rendering_state.signals);
  setup_rendered_canvas (self.state, getter! (rendering => rendering.signals) + Identity::rendered_getter() + getter! (rendered => rendered.rendered_after), 32);
  js!{@{& container}.append (@{self.assign_row (js!{ return @{rendered.canvas.clone()}.parent()})});}
      
  *self.rows += 1;
  
  if info.id == "harmonics" {
    let toggle = self.checkbox_input (
      "odd_harmonics",
      "Odd harmonics only",
      getter! (state => state.sound.odd_harmonics)
    );
    js!{@{&toggle}.appendTo(@{& container}).addClass("odd_harmonics_toggle")}
    *self.rows += 1;
  }
  
  let effects_shown = guard.effects_shown.contains (info.id);
  
  if signal.effects.len() > 0 {
    let id = info.id.clone();
    let view_toggle = self.assign_row (button_input (
      & format! ("{} {} {}... ▼",
        if effects_shown {"Hide"} else {"Show"},
        signal.effects.len() as i32,
        if signal.effects.len() == 1 {"effect"} else {"effects"},
      ),
      {let state = self.state.clone(); move || {
        {
          let mut guard = state.borrow_mut();
          if !guard.effects_shown.insert (id) {guard.effects_shown.remove (id);}
        }
        redraw (& state);
      }}
    ));
    js!{@{&view_toggle}.appendTo(@{& container}).addClass("view_toggle")}
    *self.rows += 1;
  }
    
  if effects_shown {for (index, effect) in signal.effects.iter().enumerate() {
    let effect_getter = state_getter.clone() + getter!(signal => signal.effects [index]);
    let delete_button = button_input ("Delete",
      input_callback_gotten_nullary (self.state, state_getter.clone(), move | signal | {signal.effects.remove (index);})
    );
    macro_rules! effect_editors {
      (
        $([
          $Variant: ident, $variant_name: expr,
            $((
              $field: ident, $name: expr, $input_method: ident
            ))*
        ])*) => {
        match *effect {
          $(SignalEffect::$Variant {..} => {
            let header = self.assign_row(js!{ return jQuery("<div>", {class: "signal_effect effect_header"}).append (@{info.name}+" "+@{$variant_name}+": ",@{delete_button})});
            js!{@{& container}.append (@{header});}
            *self.rows += 1;
            $(
              js!{@{& container}.append (@{self.$input_method(
                & format! ("{}_{}_{}", & info.id, index, stringify! ($field)),
                $name,
                effect_getter.clone() + variant_field_getter! (SignalEffect::$Variant => $field)
              )}.addClass("signal_effect input"))}
              *self.rows += 1;
            )*
          },)*
          //_=>(),
        }
      }
    }
    effect_editors! {
      [Jump, "Jump",
        (time, "Time", time_input)
        (size, "Size", difference_input)
      ]
      [Slide, "Slide",
        (start, "Start", time_input)
        (duration, "Duration", time_input)
        (size, "Size", difference_input)
        (smooth_start, "Smooth start", checkbox_input)
        (smooth_stop, "Smooth stop", checkbox_input)
      ]
      [Oscillation, "Oscillation",
        (size, "Size", difference_input)
        (frequency, "Frequency", frequency_input)
        (waveform, "Waveform", waveform_input)
      ]
    }
  }}
  
    if signal.effects.len() > 0 {
      let sample_rate = 500.0;
      let samples = display_samples (sample_rate, max (sound.duration(), signal.draw_through_time()), | time | signal.sample (time, false));
      let canvas = canvas_of_samples (& samples, sample_rate, if effects_shown {100.0} else {32.0}, info.slider_range, sound.duration());
      js!{ @{& container}.append (@{canvas}.parent().css("grid-row", @{first_row + 1}+" / "+@{*self.rows})); }
    }
    
    js!{ @{& container}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{first_row}+" / "+@{*self.rows})); }
  }
}


pub fn display_samples <F: FnMut(f64)->f64> (sample_rate: f64, duration: f64, mut sampler: F)->Vec<f64> {
  let duration = min (duration, MAX_RENDER_LENGTH);
  let num_samples = (duration*sample_rate).ceil() as usize + 1;
  (0..num_samples).map (| sample | sampler (sample as f64/sample_rate)).collect()
}

pub fn canvas_of_samples (samples: & [f64], sample_rate: f64, canvas_height: f64, default_range: [f64; 2], target_duration: f64)->Value {
  let canvas = js!{ return $(new_canvas ());};
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
}

pub fn setup_rendered_canvas (state: &Rc<RefCell<State>>, rendered_getter: Getter <RenderingState, RenderedSamples>, height: i32) {
  let guard = state.borrow();
  let rendered = rendered_getter.get (& guard.rendering_state);
  js!{
    var canvas = @{rendered.canvas.clone()};
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
  rendered.redraw (None, & guard.rendering_state.constants);
}
