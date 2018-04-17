"use strict";


var panels = $("#panels");

var audio = window.audio = new AudioContext();
var audio_source;
var sample_rate = 44100;
window.last_edit_number = 0;


function play_buffer (buffer, offset, duration) {
  if (audio_source) {audio_source.stop();}
  audio_source = audio.createBufferSource();
  audio_source.buffer = buffer;
  audio_source.connect (audio.destination);
  audio_source.start (audio.currentTime, offset, duration);
}
