"use strict";

import init, { rust_init, do_frame, rust_mousedown, rust_mousemove, rust_mouseup, }
  from '/my-factory-has-a-trillion-machines-web-frontend/pkg/deck_of_unhealthy_defense_mechanisms.js';

const canvas = document.getElementById("canvas");
const context = canvas.getContext('2d');

var dpr = window.devicePixelRatio || 1.0;
var width = 800;
var height = 800;
var physical_width = height*dpr;
var physical_height = width*dpr;
canvas.style.width = width+"px";
canvas.style.height = height+"px";
canvas.width = physical_width;
canvas.height = physical_height;

window.clear_canvas = function () {
  context.fillStyle = "black";
  context.fillRect(0, 0, context.canvas.width, context.canvas.height);
};

window.draw_rect = function (
  cx, cy, sx, sy,
) {
  context.save();
  //context.scale(context.canvas.width, context.canvas.height);
  context.translate (cx, cy);
  context.rotate (-(Math.PI*0.5));

  var sprite = loaded_sprites[sprite];

  context.fillRect(-sx/2.0,-sy/2.0, sx,sy);

  context.restore();
};

async function run() {
  await init();
  rust_init();

  function frame() {
    window.requestAnimationFrame(frame);
    rust_do_frame();
  }
  frame();
}

run();
