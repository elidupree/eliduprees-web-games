use std::rc::Rc;
use std::cell::RefCell;
use stdweb::unstable::{TryInto, TryFrom};
use stdweb::{JsSerialize, Value};
use ordered_float::OrderedFloat;

use super::*;


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
    options: & waveforms_list(),
    getter: getter,
  }.render()
  /*let result = js!{return $("<div>", {class: "labeled_input radio"}).append (
    $("<label>", {text:@{name} + ": "}),
    @{menu_input (state, getter, &waveforms_list())}
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
      let uh: Getter <State, Signals> = getter! (state => state.sound.signals);
      let state_getter = uh + signals_getter.clone();
      let info = Identity::info();
    let signal = signals_getter.get (& guard.sound.signals);
    let first_row = *self.rows;
      
  let container = self.main_grid;
  
  let applicable = Identity::applicable (sound);
  let enabled = applicable && (signal.enabled || !info.can_disable);
  
  //js!{@{& container}.append (@{info.name} + ": ");}
  
  let mut label = if enabled {
    let initial_value_input = self.value_input (
      & format! ("{}_initial", & info.id),
      info.name,
      state_getter.clone() + getter! (signal => signal.initial_value)
    );
    js!{@{& container}.append (@{&initial_value_input})}
    //let input_height = js!{ return @{&initial_value_input}.outerHeight()};
    self.assign_row(js!{ return @{initial_value_input}.children("label");})
  } else {
    self.assign_row(js!{ return $("<span>").text (@{info.name});})
  };
  
  if applicable && info.can_disable {
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
  
  if !applicable {
    self.assign_row(js!{ return $("<span>", {class: "signal_not_applicable"}).text ("Not applicable for the current waveform").appendTo(@{& container});});
  }
  
  if enabled {
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
  
  let rendered_canvas = make_rendered_canvas(self.state, getter! (rendering: RenderingState => rendering.signals) + Identity::rendering_getter() + getter! (rendered: SignalRenderingState => rendered.rendered_after.illustration), 32);
  
  redraw.render_progress_functions.push (Rc::new (move || rendered_canvas.update ()));
  
  js!{@{& container}.append (@{self.assign_row (js!{ return @{rendered_canvas.canvas.canvas.clone()}.parent()})});}
  
  } *self.rows += 1; if enabled {
  
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
      let samples = display_samples (sample_rate, max (sound.duration(), signal.draw_through_time()), | time | 0.0/*signal.sample (time, false)*/);
      let canvas = canvas_of_samples (& samples, sample_rate, , info.slider_range, sound.duration());
      
      let signal_canvas = IllustrationCanvas::new(state.clone(), getter! (rendering: RenderingState => rendering.signals) + Identity::rendering_getter() + getter! (rendered: SignalRenderingState => rendered.illustration));
      
      js!{@{& signal_canvas.canvas.canvas} [0].height = @{if effects_shown {100.0} else {32.0}}}
      
      redraw.render_progress_functions.push (Rc::new (move || signal_canvas.update ()));
      
      js!{ @{& container}.append (@{canvas}.parent().css("grid-row", @{first_row + 1}+" / "+@{*self.rows})); }
    }
  }
    js!{ @{& container}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{first_row}+" / "+@{*self.rows})); }
  }
}

