"use strict";

import init, { rust_init, rust_do_frame, }
  from '/deck-of-unhealthy-defense-mechanisms/pkg/deck_of_unhealthy_defense_mechanisms.js';

const canvas = document.getElementById("canvas");
const context = canvas.getContext('2d');

const action_keys = {
  z: "InteractLeft",
  x: "InteractRight",
  c: {"PlayCard": 0},
  v: {"PlayCard": 1},
  b: {"PlayCard": 2},
  n: {"PlayCard": 3},
  m: {"PlayCard": 4},
};
const direction_keys = {
  w: ["vertical", -1],
  ArrowUp: ["vertical", -1],
  a: ["horizontal", -1],
  ArrowLeft: ["horizontal", -1],
  s: ["vertical", 1],
  ArrowDown: ["vertical", 1],
  d: ["horizontal", 1],
  ArrowRight: ["horizontal", 1],
};

let dpr = null;
let resized = true;
window.addEventListener("resize", () => {resized = true;});
let canvas_css_size = [0,0];
let canvas_physical_size = [0,0];
const update_canvas_size = () => {
  if (resized || dpr !== (window.devicePixelRatio || 1.0)) {
    resized = false;
    dpr = window.devicePixelRatio || 1.0;
    canvas_css_size = [window.innerWidth, window.innerHeight];
    [canvas.width, canvas.height] = canvas_physical_size = canvas_css_size.map(d => d*dpr);
    canvas.style.width = canvas_css_size[0]+"px";
    canvas.style.height = canvas_css_size[1]+"px";
  }
}

window.clear_canvas = () => {
  context.fillStyle = "black";
  context.fillRect(0, 0, context.canvas.width, context.canvas.height);
};

window.draw_rect = (cx, cy, sx, sy,) => {
  context.save();
  context.translate (cx, cy);
  context.rotate (-(Math.PI*0.5));
  context.fillStyle = "white";
  context.fillRect(-sx/2.0,-sy/2.0, sx,sy);
  context.restore();
};

const movement_intents = {
  horizontal: [0],
  vertical: [0],
};
let action_intents = [];
document.body.addEventListener("keydown", (event) => {
  const key = event.key;
  if (direction_keys[key] !== undefined) {
    const [dimension, direction] = direction_keys[key];
    if (!movement_intents[dimension].includes(direction)) {
      // in movement, the latest is given priority (pressing right overrides left)
      movement_intents[dimension].unshift(direction);
    }
  }
  if (action_keys[key] !== undefined) {
    if (!action_intents.includes(key)) {
      // for actions, the first is given priority (you can't interrupt yourself)
      action_intents.push(key);
    }
  }
});
document.body.addEventListener("keyup", (event) => {
  const key = event.key;
  if (direction_keys[key] !== undefined) {
    const [dimension, direction] = direction_keys[key];
    movement_intents[dimension] = movement_intents[dimension].filter(i => i !== direction);
  }
  if (action_keys[key] !== undefined) {
    action_intents = action_intents.filter(i => i !== key);
  }
});

async function run() {
  await init();
  rust_init();

  function frame(time) {
    window.requestAnimationFrame(frame);
    let intent;
    if (action_intents.length === 0) {
      intent = {"Move": [movement_intents.horizontal[0], movement_intents.vertical[0]]};
    } else {
      intent = {"Interact": action_keys[action_intents[0]]};
    }
    //document.getElementById("debug").innerText = JSON.stringify(action_intents);

    update_canvas_size();
    rust_do_frame(time, {
      intent,
      canvas_css_size,
      canvas_physical_size,
    });
  }
  window.requestAnimationFrame(frame);
}

run();
