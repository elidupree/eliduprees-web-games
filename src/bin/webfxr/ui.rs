use std::rc::Rc;
use std::cell::RefCell;
use std::str::FromStr;
use stdweb::unstable::{TryInto, TryFrom};
use stdweb::web::TypedArray;
use stdweb::{JsSerialize, Value};
use serde::{Serialize};
use serde::de::DeserializeOwned;
use ordered_float::OrderedFloat;

use super::*;


#[derive (Derivative)]
#[derivative (Clone (bound =""))]
pub struct Getter <T, U> {
  pub get: Rc <Fn(&T)->&U>,
  pub get_mut: Rc <Fn(&mut T)->&mut U>,
}
impl <T, U> Getter <T, U> {
  pub fn get<'a, 'b> (&'a self, value: &'b T)->&'b U {
    (self.get) (value)
  }
  pub fn get_mut<'a, 'b> (&'a self, value: &'b mut T)->&'b mut U {
    (self.get_mut) (value)
  }
}

impl <T: 'static,U: 'static,V: 'static> ::std::ops::Add<Getter <U, V>> for Getter <T, U> {
  type Output = Getter <T, V>;
  fn add (self, other: Getter <U, V>)->Self::Output {
    let my_get = self.get;
    let my_get_mut = self.get_mut;
    let other_get = other.get;
    let other_get_mut = other.get_mut;
    Getter {
      get: Rc::new (move | value | (other_get) ((my_get) (value))),
      get_mut: Rc::new (move | value | (other_get_mut) ((my_get_mut) (value))),
    }
  }
}

macro_rules! getter {
  ($value: ident => $($path:tt)*) => {
    Getter {
      get    : Rc::new (move | $value | &    $($path)*),
      get_mut: Rc::new (move | $value | &mut $($path)*),
    }
  }
}
macro_rules! variant_field_getter {
  ($Enum: ident::$Variant: ident => $field: ident) => {
    Getter {
      get    : Rc::new (| value | match value {
        &    $Enum::$Variant {ref     $field,..} => $field,
        _ => unreachable!(),
      }),
      get_mut: Rc::new (| value | match value {
        &mut $Enum::$Variant {ref mut $field,..} => $field,
        _ => unreachable!(),
      }),
    }
  }
}


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

pub fn input_callback_gotten<T, U, F> (state: &Rc<RefCell<State>>, getter: &Getter <State, T>, callback: F)->impl (Fn (U)->bool)
  where
    F: Fn(&mut T, U)->bool {
  let getter = getter.clone();
  input_callback (state, move | state, arg | (callback)(getter.get_mut (state), arg))
}

pub fn input_callback_gotten_nullary<T, F> (state: &Rc<RefCell<State>>, getter: &Getter <State, T>, callback: F)->impl (Fn ()->bool)
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
    let result = js!{return $("<div>", {class: "labeled_input"}).append (
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
      var result = $("<div>", {class: "labeled_input"}).append (
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
  pub id: & 'a str,
  pub name: & 'a str,
  pub slider_range: [f32; 2],
  pub difference_slider_range: [f32; 2],
  pub getter: Getter <State, Signal <T>>
}

impl <'a, T: UserNumberType> SignalEditorSpecification <'a, T> {
  pub fn numeric_input <U: UserNumberType> (&self, id: & str, name: & str, slider_range: [f32; 2], getter: Getter <State, UserNumber <U>>)->Value {
    let current_value = getter.get (&self.state.borrow()).clone();
    NumericalInputSpecification {
      state: self.state,
      id: id,
      name: name, 
      slider_range: slider_range,
      current_value: current_value,
      input_callback: input_callback (self.state, move | state, value: UserNumber <U> | {
        *getter.get_mut (state) = value;
        true
      }),
    }.render()
  }

  pub fn time_input (&self, id: & str, name: & str, getter: Getter <State, UserTime>)->Value {
    self.numeric_input (id, name, [0.0, 3.0], getter)
  }
  
  pub fn value_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <T>>)->Value {
    self.numeric_input (id, name, self.slider_range, getter)
  }
  
  pub fn difference_input (&self, id: & str, name: & str, getter: Getter <State, UserNumber <T::DifferenceType>>)->Value {
    self.numeric_input (id, name, self.difference_slider_range, getter)
  }
  
  pub fn frequency_input (&self, id: & str, name: & str, getter: Getter <State, UserFrequency>)->Value {
    self.numeric_input (id, name, [1.0f32.log2(), 20f32.log2()], getter)
  }


  pub fn render (self) {
    
      let guard = self.state.borrow();
    let signal = self.getter.get (& guard);
      
  let container = js!{ return $("<div>", {id:@{self.id}, class: "panel"});};
  js!{ $("#panels").append (@{& container});}
  
  
  js!{@{& container}.append (@{self.name} + ": ");}
  if !signal.constant {js!{@{& container}.append (@{
    canvas_of_samples (& display_samples (& guard.sound, | time | signal.sample (time)))
  });}}
  
  let initial_value_input = self.value_input (
    & format! ("{}_initial", & self.id),
    if signal.constant {self.name} else {"Initial value"}, 
    self.getter.clone() + getter! (signal => signal.initial_value)
  );
  
  js!{@{& container}.append (@{initial_value_input})}
  
  
    
  if !signal.constant {for (index, effect) in signal.effects.iter().enumerate() {
    let effect_getter = self.getter.clone() + getter!(signal => signal.effects [index]);
    let delete_button = button_input ("Delete",
      input_callback_gotten_nullary (self.state, &self.getter, move | signal | {
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
            js!{@{& container}.append (@{$variant_name}+": ",@{delete_button});}
            $(
              js!{@{& container}.append (@{self.$input_method(
                & format! ("{}_{}_{}", & self.id, index, stringify! ($field)),
                $name,
                effect_getter.clone() + variant_field_getter! (SignalEffect::$Variant => $field)
              )})}
            )*
          },)*
          _=>(),
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
      ]
      [Oscillation, "Oscillation",
        (size, "Size", difference_input)
        (frequency, "Frequency", frequency_input)
      ]
    }
  }}
  
  let toggle_constant_button = button_input (
    if signal.constant {"Complicate"} else {"Simplify"},
    input_callback_gotten_nullary (self.state, &self.getter, move | signal | {
      signal.constant = !signal.constant;
      true
    })
  );
  
  
  js!{ @{& container}.append (@{toggle_constant_button}); }
  
  if !signal.constant {
    let range = self.difference_slider_range;
    let add_jump_button = button_input (
      "Add jump",
      input_callback_gotten_nullary (self.state, &self.getter, move | signal | {
        signal.effects.push (SignalEffect::Jump {time: UserTime::from_rendered (0.5), size: UserNumber::from_rendered (range [1])});
        true
      })
    );
    let add_slide_button = button_input (
      "Add slide",
      input_callback_gotten_nullary (self.state, &self.getter, move | signal | {
        signal.effects.push (SignalEffect::Slide {start: UserTime::from_rendered (0.5), duration: UserTime::from_rendered (0.5), size: UserNumber::from_rendered (range [1]), smooth_start: true, smooth_stop: true });
        true
      })
    );
    let add_oscillation_button = button_input (
      "Add oscillation",
      input_callback_gotten_nullary (self.state, &self.getter, move | signal | {
        signal.effects.push (SignalEffect::Oscillation {size: UserNumber::from_rendered (range [1]/3.0), frequency: UserNumber::from_rendered (4.0f32.log2()), waveform: Waveform::Square });
        true
      })
    );
    js!{ @{& container}.append (@{add_jump_button}, @{add_slide_button}, @{add_oscillation_button}); }

  }
  
  }
}

  
  /*
   macro_rules! signal_input {
      ([$signal: ident $($args: tt)*] $effect: expr) => {{
      let get_signal_mut = get_signal_mut.clone();
  input_callback! ([state $($args)*] {
    let mut guard = state.borrow_mut();
    let $signal = get_signal_mut (&mut guard) ;
    $effect
  })
      }}
    }
  
  
  
  for (index, control_point) in signal.control_points.iter().enumerate() {
    if signal.constant && index >0 {break;}
    let id = format! ("{}_{}", id, index);
    macro_rules! control_input {
      ([$control: ident $($args: tt)*] $effect: expr) => {{
      let get_signal_mut = get_signal_mut.clone();
  input_callback! ([state $($args)*] {
    let mut guard = state.borrow_mut();
    let signal = get_signal_mut (&mut guard) ;
    let $control = &mut signal.control_points [index];
    $effect
  })
      }}
    }
    
    let control_editor = js!{
      const control_editor = $("<div>");
      @{& container}.append (control_editor);
      return control_editor;
    };
    
    if index >0 {js!{
      @{& control_editor}.append (@{
        time_editor (& guard, & format! ("{}_time", &id), "Time", control_point.time,
          control_input! ([control, value: f32] control.time = value)
        )
      })
    }}
    
    js!{
      var frequency_editor = @{
        frequency_editor (& guard, & format! ("{}_frequency", &id), "Frequency", control_point.value, control_input! ([control, value: f32] control.value = value))
      };
      @{& control_editor}.append (frequency_editor) ;
      if (@{signal.constant}) {
        @{& control_editor}.css ("display", "inline");
        frequency_editor.css ("display", "inline");
      }
    }
    if !signal.constant {js!{
      if (@{index >0}) {
        var jump_editor = @{
          frequency_editor (& guard, & format! ("{}_jump", &id), "Jump to", control_point.value_after_jump, control_input! ([control, value: f32] control.value_after_jump = value))
        };
        @{& control_editor}.append (jump_editor) ;
        jump_editor.prepend (
          $("<input>", {type: "checkbox", checked:@{control_point.jump}}).on ("input", function(event) {
            @{control_input! ([control] control.jump = !control.jump)}();
          })
        );
      }
@{& control_editor}.append (numerical_input ({
  id: @{&id} + "slope",
  text: "Slope (Octaves/second)",
  min: - 10.0,
  max: 10.0,
  current:@{round_step (control_point.slope, 1000.0)},
  step: 0.01,
}, 
  @{control_input! ([control, value: f64] control.slope = value as f32)}
      )) ;
      
      if (@{index >0}) {
      var delete_callback = @{signal_input! ([signal] {
        signal.control_points.remove (index);
      })};
      @{& control_editor}.append ($("<input>", {
        type: "button",
        id: @{&id} + "delete_control",
        value: "Delete control point"
      }).click (function() {delete_callback()})
      );   
      }   

      var callback = @{signal_input! ([signal] {
        let previous = signal.control_points [index].clone();
        let next = signal.control_points.get (index + 1).cloned();
        let time = match next {
          None => previous.time + 0.5,
          Some (thingy) => (previous.time + thingy.time)/2.0,
        };
        let value = signal.sampler().sample(time);
        let offset = 0.000001;
        let offset_value = signal.sampler().sample (time + offset) ;
        let slope = (offset_value - value)/offset;
        signal.control_points.insert (index + 1, ControlPoint {
          time: time, value: value, slope: slope,
          jump: false, value_after_jump: value,
        });
      })};
      @{& container}.append ($("<input>", {
        type: "button",
        id: @{&id} + "add_control",
        value: "Add control point"
      }).click (function() {callback()})
      );
      
    }}
  }
  


      
      
      
    });
  }
}

fn add_signal_editor <T> () {
  let get_signal = Rc::new (get_signal);
  let get_signal_mut = Rc::new (get_signal_mut);
  let guard = state.borrow();
  let sound = & guard.sound;
  let signal = get_signal (&guard) ;}*/

pub fn display_samples <F: FnMut(f32)->f32> (sound: & SoundDefinition, mut sampler: F)->Vec<f32> {
  let num_samples = (sound.duration()*DISPLAY_SAMPLE_RATE).ceil() as usize + 1;
  (0..num_samples).map (| sample | sampler (sample as f32/DISPLAY_SAMPLE_RATE)).collect()
}

pub fn canvas_of_samples (samples: & [f32])->Value {
  let canvas = js!{ return document.createElement ("canvas") ;};
  let canvas_height = 100.0;
  let context = js!{
    var canvas = @{& canvas};
    canvas.width = @{samples.len() as f64};
    canvas.height = @{canvas_height};
    var context = canvas.getContext ("2d") ;
    return context;
  };
  
    let min = samples.iter().min_by_key (| value | OrderedFloat (**value)).unwrap() - 0.0001;
    let max = samples.iter().max_by_key (| value | OrderedFloat (**value)).unwrap() + 0.0001;
    let range = max - min;
    
    for (index, sample) in samples.iter().enumerate() {
      js!{
        var context =@{&context};
        var first = @{index as f32 + 0.5};
        var second = @{(max - sample)/range*canvas_height};
        if (@{index == 0}) {
          context.moveTo (first, second);
        } else {
          context.lineTo (first, second);
        }
      }
    }
    
  js!{
    var context =@{&context};
    context.stroke();
  }
  
  canvas
}
