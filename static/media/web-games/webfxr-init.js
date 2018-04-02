"use strict";


var panels = $("#panels");

var audio = new AudioContext();
var audio_source;
var sample_rate = 44100;
window.last_edit_number = 0;

var definition = {};

function play_buffer (buffer) {
  if (audio_source) {audio_source.stop();}
  audio_source = audio.createBufferSource();
  audio_source.buffer = buffer;
  audio_source.connect (audio.destination);
  audio_source.start ();
}

function synthesize_and_play () {
  const length = sample_rate*2;
  const buffer = audio.createBuffer (1, length, sample_rate);
  const data = buffer.getChannelData (0);
  play_buffer (buffer);
}

function debounced_edit_function (callback) {
  const debounced =_.debounce(function(value, last_edit_when_called) {
    if (last_edit_number === last_edit_when_called) {
      ++last_edit_number;
      callback (value);
    }
  }, 200);
  return function (value) {debounced (value, last_edit_number);};
}

window.numerical_input = function (data, update_callback) {
  const update = debounced_edit_function (update_callback);
  let input_specs = {type: "range", id: data.id+"_numerical_range", value: data.current, min: data.min, max: data.max, step: data.step };
  function to_log (value) {return 1000*(Math.log (value) - data.log_min)/data.log_range;}
  if (data.logarithmic) {
    data.log_min = Math.log (data.min);
    data.log_range = Math.log (data.max) - data.log_min;
    input_specs.min = 0;
    input_specs.max = 1000;
    input_specs.step = 1;
    input_specs.value = Math.round (to_log (data.current));
    console.log (input_specs.value) ;
  }
  const range_input = $("<input>", input_specs);
  const number_input = $("<input>", {type: "number", id: data.id+"_numerical_number", value: data.current, min: data.min, max: data.max, step: data.step });
  
  function valid (value) {
    return Number.isFinite (value) && !(data.logarithmic && value === 0);
  }
  
  // immediately update the range and number inputs with each other, even though the actual
  // data editing is debounced.
  function range_overrides() {
    let value = range_input[0].valueAsNumber;
    if (data.logarithmic) {value = data.min*Math.exp(value*data.log_range/1000);}
    if (!valid (value)) {return;}
    number_input.val(value);
    update(value);
  }
  function number_overrides() {
    const value = number_input[0].valueAsNumber;
    if (!valid (value)) {return;}
    set_range_input(value);
    update(value);
  }
  function set_range_input(value) {
    if (data.logarithmic) {
      range_input.val(to_log (value));
    }
    else {
      range_input.val(value);
    }
  }
  // the default above wasn't working for some reason, so just override it
  set_range_input (data.current) ;

  const result = $("<div>", {class: "labeled_input"}).append (
    range_input.on ("input", range_overrides),
    number_input.on ("input", number_overrides),
    $("<label>", {"for": data.id+"_numerical_number", text: data.text})
  ).on("wheel", function (event) {
    let value = number_input[0].valueAsNumber;
    value += (Math.sign(event.originalEvent.deltaY) || Math.sign(event.originalEvent.deltaX) || 0)*data.step;
    number_input.val (value);
    number_overrides ();
  });
  
  return result;
}

window.radio_input = function (data, update_callback) {
  const update = update_callback;
  const result = $("<div>", {class: "labeled_input"}).append (
    $("<label>", {text: data.text + ":"})
  );
  
  data.options.forEach(function(option) {
    result.append (
      $("<input>", {type: "radio", id: data.id+"_radios_" + option.value, name: data.id+"_radios", value: option.value, checked: option.value === data.current}).click (choice_overrides),
      $("<label>", {"for": data.id+"_radios_" + option.value, text: option.text}),
    );
  });
  
  function choice_overrides() {
    const value = $("input:radio[name="+data.id+"_radios]:checked").val();
    update(value);
  }
  function value_overrides() {
    const value = definition [data.field];
    if (value === NaN) {return;}
    result.find ("#"+data.id+"_radios_" + value).prop ("checked", true);
    update(value);
  }
  function updated(value) {
    definition [data.field] = value;
    synthesize_and_play ();
  }
  
  return result;
}

/*
    $("<div>", {class: "labeled_input"}).append (
      $("<input>", {type: "checkbox", id: id+"_enabled"}),
      $("<label>", {"for": id+"_enabled", text: "enabled"})
    ),
*/
