use std::rc::Rc;
use std::cell::RefCell;
use stdweb::unstable::{TryInto, TryFrom};
use stdweb::{JsSerialize, Value};
use ordered_float::OrderedFloat;

use super::*;


pub fn input_callback<T, F> (state: &Rc<RefCell<State>>, callback: F)->impl (Fn (T)->bool)
  where
    F: Fn(&mut State, T)->bool {
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
      redraw (&state);
    }
    sound_changed
  }
}

pub fn undo (state: &Rc<RefCell<State>>)->bool {
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
    redraw (&state);
  }
  sound_changed
}
pub fn redo (state: &Rc<RefCell<State>>)->bool {
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
    redraw (&state);
  }
  sound_changed
}

pub fn input_callback_nullary<F> (state: &Rc<RefCell<State>>, callback: F)->impl (Fn ()->bool)
  where
    F: Fn(&mut State)->bool {
  let hack = input_callback (state, move | state,()| (callback)(state));
  move || {
    (hack)(())
  }
}

pub fn input_callback_gotten<T, U, F> (state: &Rc<RefCell<State>>, getter: Getter <State, T>, callback: F)->impl (Fn (U)->bool)
  where
    F: Fn(&mut T, U)->bool {
  let getter = getter.clone();
  input_callback (state, move | state, arg | (callback)(getter.get_mut (state), arg))
}

pub fn input_callback_gotten_nullary<T, F> (state: &Rc<RefCell<State>>, getter: Getter <State, T>, callback: F)->impl (Fn ()->bool)
  where
    F: Fn(&mut T)->bool {
  let getter = getter.clone();
  input_callback_nullary (state, move | state | (callback)(getter.get_mut (state)))
}


pub fn button_input<F: 'static + Fn()->bool> (name: & str, callback: F)->Value {
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
  let callback = input_callback_gotten (state, getter, | target, value: bool | {
    *target = value;
    true
  });
  let result: Value = js!{
    var input;
    return $("<div>", {class: "labeled_input checkbox"}).append (
      input = on ($("<input>", {type: "checkbox", id: @{id}, checked:@{current_value}}), "click", function() {@{callback}(input.prop ("checked"));}),
      $("<label>", {"for": @{id}, text: @{name}})
    );
  };
  result
}

pub fn checkbox_meta_input (state: &Rc<RefCell<State>>, id: & str, name: & str, getter: Getter <State, bool>)->Value {
  let current_value = getter.get (&state.borrow()).clone();
  let state = state.clone() ;
  let callback = move | value: bool | {
    *getter.get_mut (&mut state.borrow_mut()) = value;
    true
  };
  let result: Value = js!{
    var input;
    return $("<div>", {class: "labeled_input checkbox"}).append (
      input = on ($("<input>", {type: "checkbox", id: @{id}, checked:@{current_value}}), "click", function() {@{callback}(input.prop ("checked"));}),
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
        return true
      }
      false
    })}(event.target.selectedIndex)
  })};
  menu
}

pub fn waveform_input (state: &Rc<RefCell<State>>, id: & str, name: & str, getter: Getter <State, Waveform>)->Value {
  let current_value = getter.get (&state.borrow()).clone();
  RadioInputSpecification {
    state: state, id: id, name: name,
    options: &[
      (Waveform::Sine, "Sine"),
      (Waveform::Square, "Square"),
      (Waveform::Triangle, "Triangle"),
      (Waveform::Sawtooth, "Sawtooth"),
      (Waveform::WhiteNoise, "White noise"),
    ],
    current_value: current_value,
    input_callback: input_callback_gotten (state, getter, | target, value: Waveform | {
      *target = value;
      true
    }),
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

pub struct RadioInputSpecification <'a, T: 'a, F> {
  pub state: & 'a Rc<RefCell<State>>,
  pub id: & 'a str,
  pub name: & 'a str,
  pub options: & 'a [(T, & 'a str)],
  pub current_value: T,
  pub input_callback: F,
}

impl <'a, F: 'static + Fn (T)->bool, T: Eq> RadioInputSpecification <'a, T, F>
  where
    T: JsSerialize,
    T: TryFrom<Value>,
    Value: TryInto<T>,
    <Value as TryInto<T>>::Error: ::std::fmt::Debug {
  pub fn render (self)->Value {
    let result = js!{return $("<div>", {class: "labeled_input radio"}).append (
      $("<label>", {text:@{self.name} + ":"})
    );};
    
    let update = js!{
      return function (value) {@{self.input_callback}(value)}
    };
    
    for &(ref value, name) in self.options.iter() {
      js!{
        function choice_overrides() {
          var value = $("input:radio[name="+@{self.id}+"_radios]:checked").val();
          @{&update}(value);
        }
        @{&result}.append (
          on ($("<input>", {type: "radio", id: @{self.id}+"_radios_" + @{value}, name: @{self.id}+"_radios", value: @{value}, checked: @{*value == self.current_value}}), "click", choice_overrides),
          $("<label>", {"for": @{self.id}+"_radios_" + @{value}, text: @{name}})
        );
      }
    }
    
    result
  }
}

pub struct NumericalInputSpecification <'a, T: UserNumberType, F> {
  pub state: & 'a Rc<RefCell<State>>,
  pub id: & 'a str,
  pub name: & 'a str,
  pub slider_range: [f64; 2],
  pub current_value: UserNumber <T>,
  pub input_callback: F,
}

impl <'a, F: 'static + Fn (UserNumber <T>)->bool, T: UserNumberType> NumericalInputSpecification<'a, T, F> {
  pub fn render (self)->Value {
    let value_type = T::currently_used (&self.state.borrow());
    let displayed_value = if value_type == self.current_value.value_type {self.current_value.source.clone()} else {value_type.approximate_from_rendered (self.current_value.rendered)};
    let slider_step = (self.slider_range [1] - self.slider_range [0])/1000.0;
    let input_callback = self.input_callback;
    let update_callback = { let value_type = value_type.clone(); move | value: String |{
          if let Some(value) = UserNumber::new (value_type.clone(), value) {
            if (input_callback)(value) {
              return true;
            }
          }
          false
        }};
    let update = js!{return _.debounce(function(value) {
        var success = @{update_callback} (value);
        if (!success) {
          // TODO display some sort of error message
        }
      }, 200);};
    let range_input = js!{return $("<input>", {type: "range", id: @{self.id}+"_numerical_range", value:@{self.current_value.rendered}, min:@{self.slider_range [0]}, max:@{self.slider_range [1]}, step:@{slider_step} });};
    let number_input = js!{return $("<input>", {type: "number", id: @{self.id}+"_numerical_number", value:@{displayed_value}});};
    
    let range_overrides = js!{return function () {
        var value = @{&range_input}[0].valueAsNumber;
        var source = @{{let value_type = value_type.clone(); move | value: f64 | value_type.approximate_from_rendered (value)}} (value);
        // immediately update the number input with the range input, even though the actual data editing is debounced.
        @{&number_input}.val(source);
        @{&update}(source);
      }
;};
    let number_overrides = js!{return function () {
        @{&update}(@{&number_input}.val());
      }
;};
    
    
    let result: Value = js!{
      var result = $("<div>", {class: "labeled_input numeric"}).append (
        on (@{&number_input}, "input", @{&number_overrides}),
        on (@{&range_input}, "input", @{&range_overrides}),
        $("<label>", {"for": @{self.id}+"_numerical_number", text:@{
          format! ("{} ({})", self.name, value_type.unit_name())
        }})
      );
      
      @{&range_input}.val(@{self.current_value.rendered});
      
      on (result, "wheel", function (event) {
        var value = @{&range_input}[0].valueAsNumber;
        value += (-Math.sign(event.originalEvent.deltaY) || Math.sign(event.originalEvent.deltaX) || 0)*@{slider_step*50.0};
        @{&range_input}.val (value);
        @{&range_overrides}();
        event.preventDefault();
      });
      return result;
    };
    result
  }
}


pub struct SignalEditorSpecification <'a, T: UserNumberType> {
  pub state: & 'a Rc<RefCell<State>>,
  pub info: &'a TypedSignalInfo<T>,
  pub rows: &'a mut u32,
}

impl <'a, T: UserNumberType> SignalEditorSpecification <'a, T> {
  pub fn assign_row (&self, element: Value)->Value {
    js!{@{&element}.css("grid-row", @{*self.rows}+" / span 1")};
    element
  }

  pub fn numeric_input <U: UserNumberType> (&self, id: & str, name: & str, slider_range: [f64; 2], getter: Getter <State, UserNumber <U>>)->Value {
    let current_value = getter.get (&self.state.borrow()).clone();
    self.assign_row(NumericalInputSpecification {
      state: self.state,
      id: id,
      name: name, 
      slider_range: slider_range,
      current_value: current_value,
      input_callback: input_callback (self.state, move | state, value: UserNumber <U> | {
        *getter.get_mut (state) = value;
        true
      }),
    }.render())
  }

  pub fn time_input (&self, id: & str, name: & str, getter: Getter <State, UserTime>)->Value {
    self.numeric_input (id, name, [0.0, 3.0], getter)
  }
  
  pub fn value_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <T>>)->Value {
    self.numeric_input (id, name, self.info.untyped.slider_range, getter)
  }
  
  pub fn difference_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <T::DifferenceType>>)->Value {
    self.numeric_input (id, name, [-self.info.untyped.difference_slider_range, self.info.untyped.difference_slider_range], getter)
  }
  
  pub fn frequency_input (&self, id: & str, name: & str, getter: Getter <State, UserFrequency>)->Value {
    self.numeric_input (id, name, [1.0f64.log2(), 20f64.log2()], getter)
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
    let signal = self.info.getter.get (& guard);
    let first_row = *self.rows;
      
  let container = js!{ return $("#panels");};
  
  //js!{@{& container}.append (@{self.info.untyped.name} + ": ");}
  
  let initial_value_input = self.value_input (
    & format! ("{}_initial", & self.info.untyped.id),
    self.info.untyped.name,
    self.info.getter.clone() + getter! (signal => signal.initial_value)
  );
  
  js!{@{& container}.append (@{&initial_value_input})}
  let input_height = js!{ return @{&initial_value_input}.outerHeight()};
  let mut label = self.assign_row(js!{ return @{initial_value_input}.children("label");});
  if self.info.untyped.can_disable {
    js!{@{label}.remove();}
    let toggle = self.checkbox_input (
      & format! ("{}_enabled", & self.info.untyped.id),
      self.info.untyped.name,
      self.info.getter.clone() + getter! (signal => signal.enabled)
    );
    js!{@{&toggle}.appendTo(@{& container}).addClass("signal_toggle")}
    label = self.assign_row(js!{ return @{toggle}.children("label");});
  }
  js!{@{label}.append(":").appendTo(@{& container}).addClass("toplevel_input_label")}
  
    //let range = self.info.untyped.difference_slider_range;
    let info = self.info.untyped.clone();
    let duration = sound.duration();
    let buttons = self.assign_row(js!{
      var menu = $("<select>", {class: "add_effect_buttons"}).append (
        $("<option>", {selected: true}).text("Add effect..."),
        $("<option>").text(@{self.info.untyped.name}+" jump"),
        $("<option>").text(@{self.info.untyped.name}+" slide"),
        $("<option>").text(@{self.info.untyped.name}+" oscillation")
      );
      on(menu, "change", function(event) {
        @{input_callback_gotten (self.state, self.info.getter.clone(), move | signal, index: i32 | {
          match index {
            1 => signal.effects.push (random_jump_effect (&mut rand::thread_rng(), duration, &info)),
            2 => signal.effects.push (random_slide_effect (&mut rand::thread_rng(), duration, &info)),
            3 => signal.effects.push (random_oscillation_effect (&mut rand::thread_rng(), duration, &info)),
            _ => return false,
          }
          true
        })}(event.target.selectedIndex)
      });
      return menu;
    });
    
    js!{ @{& container}.append (@{buttons}); }

    if let Some(ref rendered_getter) = self.info.rendered_getter {
      let rendered = (rendered_getter) (& guard.rendering_state);
      js!{
        var canvas = @{self.assign_row (rendered.canvas.clone()) };
        canvas[0].height =@{input_height};
        canvas[0].width =@{MAX_RENDER_LENGTH*DISPLAY_SAMPLE_RATE};
        @{& container}.append (canvas);
        on (canvas, "click", function() {@{{
          let state = self.state.clone() ;
          let getter = rendered_getter.clone();
          move || {
            let mut guard = state.borrow_mut() ;
            play (&mut guard, getter.clone());
          }
        }}();});
      }
    }
      
  *self.rows += 1;
    
  for (index, effect) in signal.effects.iter().enumerate() {
    let effect_getter = self.info.getter.clone() + getter!(signal => signal.effects [index]);
    let delete_button = button_input ("Delete",
      input_callback_gotten_nullary (self.state, self.info.getter.clone(), move | signal | {
        signal.effects.remove (index);
        true
      })
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
            let header = self.assign_row(js!{ return jQuery("<div>", {class: "signal_effect effect_header"}).append (@{self.info.untyped.name}+" "+@{$variant_name}+": ",@{delete_button})});
            js!{@{& container}.append (@{header});}
            *self.rows += 1;
            $(
              js!{@{& container}.append (@{self.$input_method(
                & format! ("{}_{}_{}", & self.info.untyped.id, index, stringify! ($field)),
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
  }
  
    if signal.effects.len() > 0 {
      let sample_rate = 500.0;
      let samples = display_samples (sample_rate, max (sound.duration(), signal.draw_through_time()), | time | signal.sample (time));
      let canvas = canvas_of_samples (& samples, sample_rate, self.info.untyped.slider_range, sound.duration());
      js!{ @{& container}.append (@{canvas}.css("grid-row", @{first_row + 1}+" / "+@{*self.rows})); }
    }
    
    js!{ @{& container}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{first_row}+" / "+@{*self.rows})); }
  }
}


pub fn display_samples <F: FnMut(f64)->f64> (sample_rate: f64, duration: f64, mut sampler: F)->Vec<f64> {
  let duration = min (duration, MAX_RENDER_LENGTH);
  let num_samples = (duration*sample_rate).ceil() as usize + 1;
  (0..num_samples).map (| sample | sampler (sample as f64/sample_rate)).collect()
}

pub fn canvas_of_samples (samples: & [f64], sample_rate: f64, default_range: [f64; 2], target_duration: f64)->Value {
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
  
  let canvas_height = 100.0;
  let display_height = | sample | (max_displayed - sample)/range_displayed*canvas_height;
  let display_x_time = | time | time*DISPLAY_SAMPLE_RATE;
  let display_x = | index | display_x_time((index as f64 + 0.5)/sample_rate);
  
  let canvas = js!{ return document.createElement ("canvas") ;};
  
  let context = js!{
    var canvas = @{& canvas};
    canvas.width = @{duration_displayed*DISPLAY_SAMPLE_RATE};
    canvas.height = @{canvas_height};
    var context = canvas.getContext ("2d") ;
    return context;
  };
  
  js!{
    var canvas = @{& canvas};
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
  
  let result: Value = js!{return $(@{& canvas});};
  result
}
