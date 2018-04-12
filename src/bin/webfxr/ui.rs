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
    let success = (callback)(&mut state.borrow_mut(), arg);
    if success {
      redraw (&state);
    }
    success
  }
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
    return $("<input>", {
      type: "button",
      value: @{name}
    }).click (function() {@{callback}();});
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
      input = $("<input>", {type: "checkbox", id: @{id}, checked:@{current_value}}).click (function() {@{callback}(input.prop ("checked"));}),
      $("<label>", {"for": @{id}, text: @{name}})
    );
  };
  result
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
    ],
    current_value: current_value,
    input_callback: input_callback_gotten (state, getter, | target, value: Waveform | {
      *target = value;
      true
    }),
  }.render()
}



//fn round_step (input: f32, step: f32)->f32 {(input*step).round()/step}

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
          $("<input>", {type: "radio", id: @{self.id}+"_radios_" + @{value}, name: @{self.id}+"_radios", value: @{value}, checked: @{*value == self.current_value}}).click (choice_overrides),
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
  pub slider_range: [f32; 2],
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
        var source = @{{let value_type = value_type.clone(); move | value: f64 | value_type.approximate_from_rendered (value as f32)}} (value);
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
        @{&range_input}.on ("input", @{&range_overrides}),
        @{&number_input}.on ("input", @{&number_overrides}),
        $("<label>", {"for": @{self.id}+"_numerical_number", text:@{
          format! ("{} ({})", self.name, value_type.unit_name())
        }})
      );
      
      @{&range_input}.val(@{self.current_value.rendered});
      
      result.on("wheel", function (event) {
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
  pub info: &'a SignalInfo,
  pub getter: Getter <State, Signal <T>>,
  pub rows: &'a mut u32,
}

impl <'a, T: UserNumberType> SignalEditorSpecification <'a, T> {
  pub fn assign_row (&self, element: Value)->Value {
    js!{@{&element}.css("grid-row", @{*self.rows}+" / span 1")};
    element
  }

  pub fn numeric_input <U: UserNumberType> (&self, id: & str, name: & str, slider_range: [f32; 2], getter: Getter <State, UserNumber <U>>)->Value {
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
    self.numeric_input (id, name, self.info.slider_range, getter)
  }
  
  pub fn difference_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <T::DifferenceType>>)->Value {
    self.numeric_input (id, name, [-self.info.difference_slider_range, self.info.difference_slider_range], getter)
  }
  
  pub fn frequency_input (&self, id: & str, name: & str, getter: Getter <State, UserFrequency>)->Value {
    self.numeric_input (id, name, [1.0f32.log2(), 20f32.log2()], getter)
  }
  
  pub fn checkbox_input (&self, id: & str, name: & str, getter: Getter <State, bool>)->Value {
    self.assign_row(checkbox_input (self.state, id, name, getter))
  }
  pub fn waveform_input (&self, id: & str, name: & str, getter: Getter <State, Waveform>)->Value {
    self.assign_row(waveform_input (self.state, id, name, getter))
  }


  pub fn render (self) {
    
      let guard = self.state.borrow();
    let signal = self.getter.get (& guard);
    let first_row = *self.rows;
      
  let container = js!{ return $("#panels");};
  
  //js!{@{& container}.append (@{self.info.name} + ": ");}
  
  let initial_value_input = self.value_input (
    & format! ("{}_initial", & self.info.id),
    self.info.name, 
    self.getter.clone() + getter! (signal => signal.initial_value)
  );
    
  let toggle_constant_button = self.assign_row(button_input (
    if signal.constant {"Complicate"} else {"Simplify"},
    input_callback_gotten_nullary (self.state, self.getter.clone(), move | signal | {
      signal.constant = !signal.constant;
      true
    })
  ));
  
  js!{@{& container}.append (@{initial_value_input}, @{toggle_constant_button}.addClass("toggle_constant"))}
  
  *self.rows += 1;
  
  if !signal.constant {
    //let range = self.info.difference_slider_range;
    let info = self.info.clone();
    let add_jump_button = button_input (
      "Add jump",
      input_callback_gotten_nullary (self.state, self.getter.clone(), move | signal | {
        signal.effects.push (random_jump_effect (&mut rand::thread_rng(), &info));
        true
      })
    );
    let info = self.info.clone();
    let add_slide_button = button_input (
      "Add slide",
      input_callback_gotten_nullary (self.state, self.getter.clone(), move | signal | {
        signal.effects.push (random_slide_effect (&mut rand::thread_rng(), &info));
        true
      })
    );
    let info = self.info.clone();
    let add_oscillation_button = button_input (
      "Add oscillation",
      input_callback_gotten_nullary (self.state, self.getter.clone(), move | signal | {
        signal.effects.push (random_oscillation_effect (&mut rand::thread_rng(), &info));
        true
      })
    );
    let buttons = self.assign_row(js!{ return $("<div>", {class: "add_effect_buttons"}).append (@{add_jump_button}, @{add_slide_button}, @{add_oscillation_button}); });
    
    js!{ @{& container}.append (@{buttons}); }
      
    *self.rows += 1;
  }

  
    
  if !signal.constant {for (index, effect) in signal.effects.iter().enumerate() {
    let effect_getter = self.getter.clone() + getter!(signal => signal.effects [index]);
    let delete_button = button_input ("Delete",
      input_callback_gotten_nullary (self.state, self.getter.clone(), move | signal | {
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
            let header = self.assign_row(js!{ return jQuery("<div>", {class: "add_effect_buttons"}).append (@{self.info.name}+" "+@{$variant_name}+": ",@{delete_button})});
            js!{@{& container}.append (@{header});}
            *self.rows += 1;
            $(
              js!{@{& container}.append (@{self.$input_method(
                & format! ("{}_{}_{}", & self.info.id, index, stringify! ($field)),
                $name,
                effect_getter.clone() + variant_field_getter! (SignalEffect::$Variant => $field)
              )}.css("grid-column", "2 / span 1"))}
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
  
    if !signal.constant && signal.effects.len() > 0 {
      js!{ @{& container}.append (@{canvas_of_samples (& display_samples (& guard.sound, | time | signal.sample (time)), self.info.slider_range)}.css("grid-row", @{first_row + 1}+" / "+@{*self.rows})); }
    }
    
  }
}


pub fn display_samples <F: FnMut(f32)->f32> (sound: & SoundDefinition, mut sampler: F)->Vec<f32> {
  let num_samples = (sound.duration()*DISPLAY_SAMPLE_RATE).ceil() as usize + 1;
  (0..num_samples).map (| sample | sampler (sample as f32/DISPLAY_SAMPLE_RATE)).collect()
}

pub fn canvas_of_samples (samples: & [f32], default_range: [f32; 2])->Value {
  let canvas = js!{ return document.createElement ("canvas") ;};
  let canvas_height = 100.0;
  let context = js!{
    var canvas = @{& canvas};
    canvas.width = @{samples.len() as f64};
    canvas.height = @{canvas_height};
    var context = canvas.getContext ("2d") ;
    return context;
  };
  
  let min_sample = *samples.iter().min_by_key (| value | OrderedFloat (**value)).unwrap();
  let max_sample = *samples.iter().max_by_key (| value | OrderedFloat (**value)).unwrap();
  let default_range_size = default_range [1] - default_range [0];
  let min_displayed = min(min_sample, default_range [0]);
  let max_displayed = max(max_sample, default_range [1]);
  let draw_min = min_sample < default_range [0] - 0.0001*default_range_size;
  let draw_max = max_sample > default_range [1] + 0.0001*default_range_size;
  let range_displayed = max_displayed - min_displayed;
  
  let display_height = | sample | (max_displayed - sample)/range_displayed*canvas_height;
  
  js!{
    var canvas = @{& canvas};
    var context =@{&context};
    context.strokeStyle = "rgb(128,128,128)";
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
    context.beginPath();
  }
    
  for (index, &sample) in samples.iter().enumerate() {
    js!{
      var context =@{&context};
      var first = @{index as f32 + 0.5};
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
