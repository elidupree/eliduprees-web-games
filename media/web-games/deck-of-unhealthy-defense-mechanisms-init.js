"use strict";

import init, { rust_init, rust_do_frame, }
  from '/deck-of-unhealthy-defense-mechanisms/pkg/deck_of_unhealthy_defense_mechanisms.js';

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
  context.translate (cx, cy);
  context.rotate (-(Math.PI*0.5));
  context.fillStyle = "white";
  context.fillRect(-sx/2.0,-sy/2.0, sx,sy);
  context.restore();
};

async function run() {
  await init();
  rust_init();

  let horizontal_intent = 0;
  let vertical_intent = 0;
  let action_intent = null;
  let action_keys = ["z","x","c","v","b","n","m"];
  document.body.addEventListener("keydown", (event) => {
    const key = event.key;
    if (key === "w" || key === "ArrowUp") {
      vertical_intent = -1;
    }
    if (key === "a" || key === "ArrowLeft") {
      horizontal_intent = -1;
    }
    if (key === "s" || key === "ArrowDown") {
      vertical_intent = 1;
    }
    if (key === "d" || key === "ArrowRight") {
      horizontal_intent = 1;
    }
    if (action_keys.includes(key)) {
      action_intent = key;
    }
  });
  document.body.addEventListener("keyup", (event) => {
    const key = event.key;
    if ((key === "w" || key === "ArrowUp") && vertical_intent === -1) {
      vertical_intent = 0;
    }
    if ((key === "a" || key === "ArrowLeft") && horizontal_intent === -1) {
      horizontal_intent = 0;
    }
    if ((key === "s" || key === "ArrowDown") && vertical_intent === 1) {
      vertical_intent = 0;
    }
    if ((key === "d" || key === "ArrowRight") && horizontal_intent === 1) {
      horizontal_intent = 0;
    }
    if (action_intent === key) {
      action_intent = null;
    }
  });

  function frame(time) {
    window.requestAnimationFrame(frame);
    let intent;
    if (action_intent === null) {
      intent = {"Move": [horizontal_intent, vertical_intent]};
    } else {
      intent = {"Interact": "InteractLeft"};
    }
    document.getElementById("debug").innerText = action_intent;
    rust_do_frame(time, {intent});
  }
  window.requestAnimationFrame(frame);
}

run();
