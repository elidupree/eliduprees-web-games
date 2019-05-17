use std::cell::RefCell;
use std::rc::Rc;
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::{JsSerialize, Value};
use typed_html::elements::FlowContent;
use typed_html::{html, text};
use stdweb::web::event::{ChangeEvent};

use super::*;

fn get<G: 'static + GetterBase<From = State, To = T>, T>(getter: &Getter<G>) -> T {
  with_state(|state| getter.get(state).clone());
}
fn set<G: 'static + GetterBase<From = State, To = T>, T>(getter: &Getter<G>, value: T) {
  with_state_mut(|state| *getter.get_mut(state) = value);
} 


pub fn checkbox_input<Builder: UIBuilder, G: 'static + GetterBase<From = State, To = bool>>(
  builder: &mut Builder,
  id: &str,
  name: &str,
  getter: Getter<G>,
) -> (Element, Element) {
  let current_value = get(&getter);
  (
    html! { <input type="checkbox" id=id checked=current_value /> },
    html! { <label for=id text=name /> },
    move |thingy| {
      thingy.add_event_listener(id, move |_: ClickEvent| {
        set(&getter, js_unwrap! {$("#"+@{id}).prop ("checked")});
      })
    },
  )
}

pub fn menu_input<Builder: UIBuilder, T: 'static + Eq + Clone, G: 'static + GetterBase<From = State, To = T>>(
  builder: &mut Builder,
  id: &str,
  getter: Getter<G>,
  options: &[(T, &str)],
) -> Element {
  let current_value = get(&getter);
  let mut values = options.iter().map(|(a, _)| a.clone()).collect();
  (
    html! {
      <select id=id>
        {options.iter().map(|(value, name)| html!{
          <option selected={*value == current_value}>
            {text!(name)}
          </option>
        })}
      </select>
    },
    move |thingy| {
      thingy.add_event_listener(id, move |_: ChangeEvent| {
        let index = js_unwrap! {$("#"+@{id}).prop ("selectedIndex")};
        if let Some(value) = values.get(index as usize) {
          set(&getter, value.clone());
        }
      })
    },
  )
}

pub fn waveform_input<Builder: UIBuilder, G: 'static + GetterBase<From = State, To = Waveform>>(
  builder: &mut Builder,
  id: &str,
  name: &str,
  getter: Getter<G>,
) -> Element {
  RadioInputSpecification {
    builder: builder,
    id: id,
    name: name,
    options: &waveforms_list(),
    getter: getter.dynamic(),
  }
  .render()
}

//fn round_step (input: f64, step: f64)->f64 {(input*step).round()/step}

pub struct RadioInputSpecification<'a, Builder: UIBuilder, T: 'a> {
  pub builder: &'a mut Builder,
  pub id: &'a str,
  pub name: &'a str,
  pub options: &'a [(T, &'a str)],
  pub getter: DynamicGetter<State, T>,
}

impl<'a, Builder: UIBuilder, T: Clone + Eq + JsSerialize + 'static> RadioInputSpecification<'a, Builder, T> {
  fn value_id(&self, value: &T) -> String {
    js_unwrap! {@{self.id}+"_radios_"+@{value}}
  }
  pub fn render(self) -> (Element, Element) {
    let current_value = get(&self.getter);
    for (value, name) in self.options {
          let id = self.value_id(value);
          let value = value.clone();
          let getter = self.getter.clone();
          self.builder.add_event_listener(&id, move |_: ClickEvent| {
            set(getter, value.clone());
          });
        }
        
    (
      html! {
        <div class="radio">
          {self.options.iter().map (| (value, name) | html!{
            <input type="button" id=self.value_id(value) value=name class={if *value == current_value {"down"} else {""}}/>
          })}
        </div>
      },
      html! { <label for=self.id text=self.name /> },
    )
  }
}

pub fn numerical_input<
  Builder: UIBuilder,
  T: UserNumberType,
  G: 'static + GetterBase<From = State, To = UserNumber<T>>,
>(
  builder: &mut Builder,
  id: &str,
  name: &str,
  getter: Getter<G>,
  slider_range: [f64; 2],
  slider_step: f64,
) -> Value {
  let current_value = get(getter);
  NumericalInputSpecification {
    id,
    name,
    slider_range,
    slider_step,
    current_value,
    input_callback: move |value: UserNumber<T>| {
      set(&getter, value)
    },
  }
  .render()
}

pub struct NumericalInputSpecification<'a, Builder: UIBuilder, T: UserNumberType, F> {
  pub builder: &'a mut Builder,
  pub id: &'a str,
  pub name: &'a str,
  pub slider_range: [f64; 2],
  pub slider_step: f64,
  pub current_value: UserNumber<T>,
  pub input_callback: F,
}

impl<'a, Builder: UIBuilder, F: 'static + Fn(UserNumber<T>) + Copy, T: UserNumberType>
  NumericalInputSpecification<'a, Builder, T, F>
{
  pub fn render(self) -> (Element, Element) {
    let value_type = with_state(T::currently_used);
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

    let number_id = format!("{}_numerical_number", self.id);
    let range_id = format!("{}_numerical_range", self.id);

    let input_callback = self.input_callback;
    let update = move |value: String| {
      if let Some(value) = UserNumber::new(value_type.clone(), value) {
        (input_callback)(value);
      }
    };

    let range_overrides = move || {
      let value = js_unwrap! {$("#"+@{range_id})[0].valueAsNumber};
      let source = value_type.approximate_from_rendered(value);
      (update)(source);
    };
    let number_overrides = move || {
      (update)(js_unwrap! {$("#"+@{number_id}).val()});
    };

    let label = format!("{} ({})", self.name, value_type.unit_name());
    
    self.builder.add_event_listener(&number_id, move |_: ChangeEvent| {
      (number_overrides)();
    });
    
    self.builder.add_event_listener(&range_id, move |_: ChangeEvent| {
          let value = js_unwrap! {$("#"+@{range_id})[0].valueAsNumber};
          let source = value_type.approximate_from_rendered(value);
          (update)(source);
        });
    
    self.builder.add_event_listener_by_name(&range_id, "wheel", move | event: Value | {
          js! {
        if (window.webfxr_scrolling) {return;}
        var event = @{event};
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
    };
        });
    

    (
      html! {
        <div id=self.id class="labeled_input numeric">
          <input type="number" id=number_id value=displayed_value min={self.slider_range [0]} max={self.slider_range [1]} step=slider_step />
          <input type="range" id=range_id value=self.current_value.rendered />
        </div>
      },
      html! {
        <label for=number_id text=label />
      },
    )
  }
}

pub struct SignalEditorSpecification<'a, Builder: UIBuilder, Identity: SignalIdentity> {
  pub builder: &'a mut Builder,
  pub redraw: &'a mut RedrawState,
  pub _marker: PhantomData<Identity>,
}

impl<'a, Builder: UIBuilder, Identity: SignalIdentity> SignalEditorSpecification<'a, Builder, Identity> {
  pub fn assign_row(&self, element: Element) -> Element {
    js! {@{&element}.css("grid-row", @{self.redraw.rows}+" / span 1")};
    element
  }

  pub fn time_input<G: 'static + GetterBase<From = State, To = UserTime>>(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Element {
    numerical_input(self.builder, id, name, [0.0, 3.0], 0.0, getter)
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
    numerical_input(self.builder, id, name, info.slider_range, info.slider_step, getter)
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
    numerical_input(
      self.builder,
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
    numerical_input(self.builder, id, name, [1.0f64.log2(), 20f64.log2()], 0.0, getter)
  }

  pub fn checkbox_input<G: 'static + GetterBase<From = State, To = bool>>(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    self.assign_row(checkbox_input(self.builder, id, name, getter))
  }
  pub fn waveform_input<G: 'static + GetterBase<From = State, To = Waveform>>(
    &self,
    id: &str,
    name: &str,
    getter: Getter<G>,
  ) -> Value {
    self.assign_row(waveform_input(self.builder, id, name, getter))
  }

  pub fn render(self) -> Vec<Element> {
    with_state(|state| {
      let sound = &state.sound;
      let signals_getter = Identity::definition_getter();
      let uh = getter! (state: State => Signals {state.sound.signals});
      let state_getter = uh + signals_getter.clone();
      let info = Identity::info();
      let signal = signals_getter.get(&sound.signals);
      let first_row = self.redraw.rows;

      let container = &self.redraw.main_grid;

      let applicable = Identity::applicable(sound);
      let enabled = sound.enabled::<Identity>();

      //js!{@{& container}.append (@{info.name} + ": ");}

      let mut elements = Vec::new();
      let mut build_functions = Vec::new();

      let signal_class = format!("{}", info.id);
      let effects_class = format!("{}_effects", info.id);

      let mut signal_label = if enabled {
        let (input, label, build) = self.value_input (
      & format! ("{}_initial", & info.id),
      info.name,
      state_getter.clone() + getter! {self@        <[NumberType: UserNumberType]>{_marker: PhantomData<NumberType> = PhantomData,} => signal: Signal<NumberType> => UserNumber<NumberType> {signal.initial_value}}
    );
        elements.append(html! { <div class=[&signal_class, "signal_numerical"]>{input}</div> });
        build_functions.append(Box::new(build));
        label
      } else {
        html! { <span>{info.name}</span> }
      };

      if applicable && info.can_disable {
        let (toggle, label, build) = self.checkbox_input (
      & format! ("{}_enabled", & info.id),
      info.name,
      state_getter.clone() + getter! (self@ <[NumberType: UserNumberType]>{_marker: PhantomData<NumberType> = PhantomData,} => signal: Signal<NumberType> => bool {signal.enabled})
    );
        elements.append(html! { <div class=[&signal_class, "signal_toggle"]>{toggle}</div> });
        build_functions.append(build);
        signal_label = label;
      }

      elements.insert(
        0,
        html! { <div class=[&signal_class, "signal_label"]>{signal_label}</div> },
      );

      if !applicable {
        elements.append (html!{ <div class=[&signal_class, "signal_not_applicable"]>"Not applicable for the current waveform"</div> });
      }

      if enabled {
        {
          let info = info.clone();
          let duration = sound.envelope.duration();
          let getter = state_getter.clone();
          let select_id = format!("{}_add_effect", info.id);

          elements.append(html! {
            <select id=select_id class=[&signal_class, "add_effect_buttons"]>
              <option selected=true>"Add effect..."</option>
              <option>{text!("{} jump", {info.name})}</option>
              <option>{text!("{} slide", info.name)}</option>
              <option>{text!("{} oscillation", info.name)}</option>
            </select>
          });
          build_functions.append(Box::new(move |thingy| {
            thingy.add_event_listener(&select_id, move |_: ChangeEvent| {
              let index = js_unwrap! {$("#"+@{select_id})[0].selectedIndex};
              with_state_mut(|state| {
                let signal = getter.get_mut(state);
                match index {
                  1 => signal.effects.push(random_jump_effect(
                    &mut rand::thread_rng(),
                    duration,
                    &info,
                  )),
                  2 => signal.effects.push(random_slide_effect(
                    &mut rand::thread_rng(),
                    duration,
                    &info,
                  )),
                  3 => signal.effects.push(random_oscillation_effect(
                    &mut rand::thread_rng(),
                    duration,
                    &info,
                  )),
                  _ => (),
                }
              });
            });
          }));
        }

        let samples_getter = getter! (rendering: RenderingState => SignalsRenderingState {rendering.signals})
          + Identity::rendering_getter()
          + getter! (rendered: SignalRenderingState => RenderedSamples {rendered.rendered_after});

        let (rendered_canvas, rendered_canvas_builder) =
          make_rendered_canvas(self.state, samples_getter, 32);
        elements
          .append(html! { <div class=[&signal_class, "rendered_canvas"]>{rendered_canvas}</div> });

        //TODO move to canvas code: self.redraw.render_progress_functions.push (Box::new (move | state | rendered_canvas.update (state)));

        if info.id == "harmonics" {
          let toggle = self.checkbox_input(
            "odd_harmonics",
            "Odd harmonics only",
            getter! (state: State => bool {state.sound.odd_harmonics}),
          );
          js! {@{&toggle}.appendTo(@{& container}).addClass("odd_harmonics_toggle")}
          self.redraw.rows += 1;
        }

        let effects_shown = state.effects_shown.contains(info.id);

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
          signal_canvas.update(state);
          self
            .redraw
            .render_progress_functions
            .push(Box::new(move |state| signal_canvas.update(state)));
        }
      }
      js! { @{& container}.prepend ($("<div>", {class:"input_region"}).css("grid-row", @{first_row}+" / "+@{self.redraw.rows})); }
    })
  }
}
