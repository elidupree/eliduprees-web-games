use std::cell::RefCell;
use std::rc::Rc;
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::{JsSerialize, Value};
//use ordered_float::OrderedFloat;

use super::*;

pub fn button_input<F: 'static + Fn()>(name: &str, callback: F) -> Value {
  let result: Value = js! {
    return on ($("<input>", {
      type: "button",
      value: @{name}
    }), "click", function() {@{callback}();});
  };
  result
}

pub fn checkbox_input<G: 'static + GetterBase<From = State, To = bool>>(
  state: &Rc<RefCell<State>>,
  id: &str,
  name: &str,
  getter: Getter<G>,
) -> Value {
  let current_value = getter.get(&state.borrow()).clone();
  let callback = input_callback_gotten(state, getter, |target, value: bool| *target = value);
  let result: Value = js! {
    return ($("<div>", {class: "labeled_input checkbox"}).append (
      on ($("<input>", {type: "checkbox", id: @{id}, checked:@{current_value}}), "click", function(event) {@{callback}($(event.target).prop ("checked"));}),
      $("<label>", {"for": @{id}, text: @{name}})
    ));
  };
  result
}

pub fn menu_input<T: 'static + Eq + Clone, G: 'static + GetterBase<From = State, To = T>>(
  state: &Rc<RefCell<State>>,
  getter: Getter<G>,
  options: &[(T, &str)],
) -> Value {
  let current_value = getter.get(&state.borrow()).clone();
  let menu = js! {
    return ($("<select>"));
  };
  let mut values = Vec::with_capacity(options.len());
  for &(ref value, name) in options.iter() {
    values.push(value.clone());
    js! {@{& menu}.append ($("<option>", {selected:@{*value == current_value}}).text(@{name}));}
  }
  js! { on (@{& menu}, "change", function(event) {
    @{input_callback_gotten (state, getter, move | target, index: i32 | {
      if let Some(value) = values.get (index as usize) {
        *target = value.clone();
      }
    })}(event.target.selectedIndex)
  })};
  menu
}

pub fn waveform_input<G: 'static + GetterBase<From = State, To = Waveform>>(
  state: &Rc<RefCell<State>>,
  id: &str,
  name: &str,
  getter: Getter<G>,
) -> Value {
  RadioInputSpecification {
    state: state,
    id: id,
    name: name,
    options: &waveforms_list(),
    getter: getter.dynamic(),
  }
  .render()
  /*let result = js!{return ($("<div>", {class: "labeled_input radio"}).append (
    $("<label>", {text:@{name} + ": "}),
    @{menu_input (state, getter, &waveforms_list())}
  ));};
  result*/
}

//fn round_step (input: f64, step: f64)->f64 {(input*step).round()/step}

pub struct RadioInputSpecification<'a, T: 'a> {
  pub state: &'a Rc<RefCell<State>>,
  pub id: &'a str,
  pub name: &'a str,
  pub options: &'a [(T, &'a str)],
  pub getter: DynamicGetter<State, T>,
}

impl<'a, T: Clone + Eq + 'static> RadioInputSpecification<'a, T>
where
  T: JsSerialize,
  T: TryFrom<Value>,
  Value: TryInto<T>,
  <Value as TryInto<T>>::Error: ::std::fmt::Debug,
{
  pub fn render(self) -> Value {
    let current_value = self.getter.get(&self.state.borrow()).clone();
    let result = js! {return ($("<div>", {class: "labeled_input radio"}).append (
      $("<label>", {text:@{self.name} + ":"})
    ));};

    let update = js! {
      return function (value) {@{input_callback_gotten (self.state, self.getter, | target, value: T | *target = value)}(value)}
    };

    for &(ref value, name) in self.options.iter() {
      let input = js! { return on ($("<input>", {type: "button", id: @{self.id}+"_radios_" + @{&value}, value: @{name}}), "click", function() {@{& update} (@{&value})});};
      if *value == current_value {
        js! { @{&input}.addClass("down"); }
      }
      js! { @{&result}.append (@{input}); }
    }

    result
  }
}

pub fn numerical_input<
  T: UserNumberType,
  G: 'static + GetterBase<From = State, To = UserNumber<T>>,
>(
  state: &Rc<RefCell<State>>,
  id: &str,
  name: &str,
  getter: Getter<G>,
  slider_range: [f64; 2],
) -> Value {
  let current_value = getter.get(&state.borrow()).clone();
  NumericalInputSpecification {
    state: state,
    id: id,
    name: name,
    slider_range: slider_range,
    slider_step: 0.0,
    current_value: current_value,
    input_callback: input_callback_gotten(state, getter, |target, value: UserNumber<T>| {
      *target = value
    }),
  }
  .render()
}

pub struct NumericalInputSpecification<'a, T: UserNumberType, F> {
  pub state: &'a Rc<RefCell<State>>,
  pub id: &'a str,
  pub name: &'a str,
  pub slider_range: [f64; 2],
  pub slider_step: f64,
  pub current_value: UserNumber<T>,
  pub input_callback: F,
}

impl<'a, F: 'static + Fn(UserNumber<T>), T: UserNumberType> NumericalInputSpecification<'a, T, F> {
  pub fn render(self) -> Value {
    let value_type = T::currently_used(&self.state.borrow());
    let displayed_value = if value_type == self.current_value.value_type {
      self.current_value.source.clone()
    } else {
      value_type.approximate_from_rendered(self.current_value.rendered)
    };
    let slider_step = if self.slider_step != 0.0 {
      self.slider_step
    } else {
      (self.slider_range[1] - self.slider_range[0]) / 1000.0
    };
    let input_callback = self.input_callback;
    let update_callback = {
      let value_type = value_type.clone();
      move |value: String| {
        if let Some(value) = UserNumber::new(value_type.clone(), value) {
          (input_callback)(value);
        }
      }
    };
    let to_rendered_callback = {
      let value_type = value_type.clone();
      move |value: String| {
        if let Some(value) = UserNumber::new(value_type.clone(), value) {
          value.rendered
        } else {
          std::f64::NAN
        }
      }
    };
    let update = js! {return (_.debounce(function(value) {
      var success = @{update_callback} (value);
      if (!success) {
        // TODO display some sort of error message
      }
    }, 200));};
    let range_input = js! {return ($("<input>", {type: "range", id: @{self.id}+"_numerical_range", value:@{self.current_value.rendered}, min:@{self.slider_range [0]}, max:@{self.slider_range [1]}, step:@{slider_step} }));};
    let number_input = js! {return ($("<input>", {type: "number", id: @{self.id}+"_numerical_number", value:@{displayed_value}}));};

    let range_overrides = js! {return function (parent) {
            var value = parent.children ("input[type=range]")[0].valueAsNumber;
            var source = @{{let value_type = value_type.clone(); move | value: f64 | value_type.approximate_from_rendered (value)}} (value);
            // immediately update the number input with the range input, even though the actual data editing is debounced.
            parent.children ("input[type=number]").val(source);
            @{&update}(source);
          }
    ;};
    let number_overrides = js! {return function (parent) {
            @{&update}(parent.children ("input[type=number]").val());
          }
    ;};

    let result: Value = js! {
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
        //console.log (event.originalEvent.deltaY, event.originalEvent.deltaX, event.originalEvent.deltaMode);
        var increment = ((-event.originalEvent.deltaY) || event.originalEvent.deltaX || 0)*@{slider_step*0.5};
        if (event.originalEvent.deltaMode === 1) {increment *= 18;}
        if (event.originalEvent.deltaMode === 2) {increment *= 400;}
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

pub struct SignalEditorSpecification<'a, Identity: SignalIdentity> {
  pub state: &'a Rc<RefCell<State>>,
  pub redraw: &'a mut RedrawState,
  pub _marker: PhantomData<Identity>,
}

impl<'a, Identity: SignalIdentity> SignalEditorSpecification<'a, Identity> {
  pub fn assign_row(&self, element: Value) -> Value {
    js! {@{&element}.css("grid-row", @{self.redraw.rows}+" / span 1")};
    element
  }

  pub fn numeric_input<
    U: UserNumberType,
    G: 'static + GetterBase<From = State, To = UserNumber<U>>,
  >(
    &self,
    id: &str,
    name: &str,
    slider_range: [f64; 2],
    slider_step: f64,
    getter: Getter<G>,
  ) -> Value {
    let current_value = getter.get(&self.state.borrow()).clone();
    self.assign_row(
      NumericalInputSpecification {
        state: self.state,
        id: id,
        name: name,
        slider_range: slider_range,
        slider_step: slider_step,
        current_value: current_value,
        input_callback: input_callback(self.state, move |state, value: UserNumber<U>| {
          *getter.get_mut(state) = value
        }),
      }
      .render(),
    )
  }

  pub fn time_input<G: 'static + GetterBase<From = State, To = UserTime>>(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    self.numeric_input(id, name, [0.0, 3.0], 0.0, getter)
  }

  pub fn value_input<
    G: 'static + GetterBase<From = State, To = UserNumber<Identity::NumberType>>,
  >(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    let info = Identity::info();
    self.numeric_input(id, name, info.slider_range, info.slider_step, getter)
  }

  pub fn difference_input<
    G: 'static
      + GetterBase<
        From = State,
        To = UserNumber<<Identity::NumberType as UserNumberType>::DifferenceType>,
      >,
  >(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    let info = Identity::info();
    self.numeric_input(
      id,
      name,
      [-info.difference_slider_range, info.difference_slider_range],
      0.0,
      getter,
    )
  }

  pub fn frequency_input<G: 'static + GetterBase<From = State, To = UserFrequency>>(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    self.numeric_input(id, name, [1.0f64.log2(), 20f64.log2()], 0.0, getter)
  }

  pub fn checkbox_input<G: 'static + GetterBase<From = State, To = bool>>(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    self.assign_row(checkbox_input(self.state, id, name, getter))
  }
  pub fn waveform_input<G: 'static + GetterBase<From = State, To = Waveform>>(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    self.assign_row(waveform_input(self.state, id, name, getter))
  }

  pub fn render(self) {
    let guard = self.state.borrow();
    let sound = &guard.sound;
    let signals_getter = Identity::definition_getter();
    let uh = getter! (state: State => Signals {state.sound.signals});
    let state_getter = uh + signals_getter.clone();
    let info = Identity::info();
    let signal = signals_getter.get(&guard.sound.signals);
    let first_row = self.redraw.rows;

    let container = &self.redraw.main_grid;

    let applicable = Identity::applicable(sound);
    let enabled = sound.enabled::<Identity>();

    //js!{@{& container}.append (@{info.name} + ": ");}

    let mut label = if enabled {
      let initial_value_input = self.value_input (
      & format! ("{}_initial", & info.id),
      info.name,
      state_getter.clone() + getter! {self@        <[NumberType: UserNumberType]>{_marker: PhantomData<NumberType> = PhantomData,} => signal: Signal<NumberType> => UserNumber<NumberType> {signal.initial_value}}
    );
      js! {@{& container}.append (@{&initial_value_input})}
      //let input_height = js!{ return @{&initial_value_input}.outerHeight()};
      self.assign_row(js! { return @{initial_value_input}.children("label");})
    } else {
      self.assign_row(js! { return ($("<span>").text (@{info.name}));})
    };

    if applicable && info.can_disable {
      js! {@{label}.remove();}
      let toggle = self.checkbox_input (
      & format! ("{}_enabled", & info.id),
      info.name,
      state_getter.clone() + getter! (self@ <[NumberType: UserNumberType]>{_marker: PhantomData<NumberType> = PhantomData,} => signal: Signal<NumberType> => bool {signal.enabled})
    );
      js! {@{&toggle}.appendTo(@{& container}).addClass("signal_toggle")}
      label = self.assign_row(js! { return @{toggle}.children("label");});
    }
    js! {@{label}.append(":").appendTo(@{& container}).addClass("toplevel_input_label")}

    if !applicable {
      self.assign_row(js!{ return ($("<span>", {class: "signal_not_applicable"}).text ("Not applicable for the current waveform").appendTo(@{& container}));});
    }

    if enabled {
      {
        let info = info.clone();
        let duration = sound.envelope.duration();
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

        js! { @{& container}.append (@{buttons}); }
      }

      let mut rendered_canvas = make_rendered_canvas(
        self.state,
        getter! (rendering: RenderingState => SignalsRenderingState {rendering.signals})
          + Identity::rendering_getter()
          + getter! (rendered: SignalRenderingState => RenderedSamples {rendered.rendered_after}),
        32,
      );

      js! {@{& container}.append (@{self.assign_row (js!{ return @{rendered_canvas.canvas.canvas.clone()}.parent()})});}
      self
        .redraw
        .render_progress_functions
        .push(Box::new(move |state| rendered_canvas.update(state)));
    }
    self.redraw.rows += 1;
    if enabled {
      if info.id == "harmonics" {
        let toggle = self.checkbox_input(
          "odd_harmonics",
          "Odd harmonics only",
          getter! (state: State => bool {state.sound.odd_harmonics}),
        );
        js! {@{&toggle}.appendTo(@{& container}).addClass("odd_harmonics_toggle")}
        self.redraw.rows += 1;
      }

      let effects_shown = guard.effects_shown.contains(info.id);

      if signal.effects.len() > 0 {
        let id = info.id.clone();
        let view_toggle = self.assign_row(button_input(
          &format!(
            "{} {} {}... ▼",
            if effects_shown { "Hide" } else { "Show" },
            signal.effects.len() as i32,
            if signal.effects.len() == 1 {
              "effect"
            } else {
              "effects"
            },
          ),
          {
            let state = self.state.clone();
            move || {
              {
                let mut guard = state.borrow_mut();
                if !guard.effects_shown.insert(id) {
                  guard.effects_shown.remove(id);
                }
              }
              redraw_app(&state);
            }
          },
        ));
        js! {@{&view_toggle}.appendTo(@{& container}).addClass("view_toggle")}
        self.redraw.rows += 1;
      }

      if effects_shown {
        for (index, effect) in signal.effects.iter().enumerate() {
          let effect_getter = state_getter.clone()
            + getter!(self@ <[NumberType: UserNumberType]>{_marker: PhantomData<NumberType> = PhantomData, index: usize = index,} => signal: Signal<NumberType> => SignalEffect<NumberType> {signal.effects [self.index]});
          let delete_button = button_input(
            "Delete",
            input_callback_gotten_nullary(self.state, state_getter.clone(), move |signal| {
              signal.effects.remove(index);
            }),
          );
          macro_rules! effect_editors {
      (
        $([
          $Variant: ident, $variant_name: expr,
            $((
              $field: ident, $name: expr, $input_method: ident, $getter: expr
            ))*
        ])*) => {
        match *effect {
          $(SignalEffect::$Variant {..} => {
            let header = self.assign_row(js!{ return jQuery("<div>", {class: "signal_effect effect_header"}).append (@{info.name}+" "+@{$variant_name}+": ",@{delete_button})});
            js!{@{& container}.append (@{header});}
            self.redraw.rows += 1;
            $(
              js!{@{& container}.append (@{self.$input_method(
                & format! ("{}_{}_{}", & info.id, index, stringify! ($field)),
                $name,
                effect_getter.clone() + $getter
              )}.addClass("signal_effect input"))}
              self.redraw.rows += 1;
            )*
          },)*
          //_=>(),
        }
      }
    }
          effect_editors! {
            [Jump, "Jump",
              (time, "Time", time_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Jump => time: UserTime))
              (size, "Size", difference_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Jump => size: UserNumber<NumberType::DifferenceType>))
            ]
            [Slide, "Slide",
              (start, "Start", time_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Slide => start: UserTime))
              (duration, "Duration", time_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Slide => duration: UserTime))
              (size, "Size", difference_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Slide => size: UserNumber<NumberType::DifferenceType>))
              (smooth_start, "Smooth start", checkbox_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Slide => smooth_start: bool))
              (smooth_stop, "Smooth stop", checkbox_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Slide => smooth_stop: bool))
            ]
            [Oscillation, "Oscillation",
              (size, "Size", difference_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Oscillation => size: UserNumber<NumberType::DifferenceType>))
              (frequency, "Frequency", frequency_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Oscillation => frequency: UserFrequency))
              (waveform, "Waveform", waveform_input, variant_field_getter! (<[NumberType: UserNumberType]>SignalEffect<NumberType> =>::Oscillation => waveform: Waveform))
            ]
          }
        }
      }

      if signal.effects.len() > 0 {
        //let sample_rate = 500.0;
        //let samples = display_samples (sample_rate, max (sound.duration(), signal.draw_through_time()), | time | 0.0/*signal.sample (time, false)*/);
        //let canvas = canvas_of_samples (& samples, sample_rate, , info.slider_range, sound.duration());

        let mut signal_canvas = IllustrationCanvas::new(
          self.state.clone(),
          (getter! (rendering: RenderingState => SignalsRenderingState {rendering.signals})
            + Identity::rendering_getter()
            + getter! (rendered: SignalRenderingState => Illustration {rendered.illustration}))
          .dynamic(),
        );

        js! {@{& signal_canvas.canvas.canvas} [0].height = @{if effects_shown {100.0} else {32.0}}}
        js! { @{& container}.append (@{& signal_canvas.canvas.canvas}.parent().css("grid-row", @{first_row + 1}+" / "+@{self.redraw.rows})); }
        signal_canvas.reset();
        signal_canvas.update(&guard);
        self
          .redraw
          .render_progress_functions
          .push(Box::new(move |state| signal_canvas.update(state)));
      }
    }
    js! { @{& container}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{first_row}+" / "+@{self.redraw.rows})); }
  }
}
