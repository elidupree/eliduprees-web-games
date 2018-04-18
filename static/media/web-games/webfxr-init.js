"use strict";


window.Module = window.Module || {};
Module.TOTAL_MEMORY = 1 << 28;

var panels = $("#panels");

var audio = window.audio = new AudioContext();
var audio_source;
var sample_rate = 44100;
window.last_edit_number = 0;

window.webfxr_callbacks = {};
window.webfxr_next_serial_number = 0;


function clear_callbacks () {window.webfxr_callbacks = {};}
function on (jQuery_object, event_name, callback) {
  const serial_number = window.webfxr_next_serial_number++;
  window.webfxr_callbacks [serial_number] = callback;
  const data_index = "data-" + event_name + "_callback_handler";
  return jQuery_object.attr (data_index, serial_number).on (event_name, function(event) {
    let target = event.target;
    let serial_number = $(target).attr (data_index);
    while (serial_number === undefined) {
      target = target.parentElement;
      serial_number = $(target).attr (data_index);
    }
    //console.log (data_index, serial_number, event.target);
    return window.webfxr_callbacks [serial_number] (event);
  });
}

function new_canvas () {
  let result = document.createElement ("canvas");
  result.id = "canvas_" + (window.webfxr_next_serial_number++);
  return result;
}

function play_buffer (buffer, offset, duration) {
  if (audio_source) {audio_source.stop();}
  audio_source = audio.createBufferSource();
  audio_source.buffer = buffer;
  audio_source.connect (audio.destination);
  audio_source.start (audio.currentTime, offset, duration);
}


window.hidden_classes = {};
window.class_is_hidden = function (cssclass) {
  let data = window.hidden_classes [cssclass];
  if (data === undefined) {return true;}
  return data;
}
window.update_class_hidden = function (container, cssclass, force) {
  if (force !== undefined) {
    if (force === "toggle") { window.hidden_classes [cssclass] = !class_is_hidden (cssclass);}
    else { window.hidden_classes [cssclass] = force;}
  }
  let hidden = class_is_hidden (cssclass);
  let members = container.find ("." + cssclass);
  if (hidden) {members.hide() ;}
  else {members.show() ;}
};

