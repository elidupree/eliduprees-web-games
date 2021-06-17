"use strict";

import init, { rust_init, rust_do_frame, }
  from '/deck-of-unhealthy-defense-mechanisms/pkg/deck_of_unhealthy_defense_mechanisms.js';

const canvas = document.getElementById("canvas");
const context = canvas.getContext('2d');

const action_keys = {
  KeyZ: "InteractLeft",
  KeyX: "InteractRight",
  KeyC: {"PlayCard": 0},
  KeyV: {"PlayCard": 1},
  KeyB: {"PlayCard": 2},
  KeyN: {"PlayCard": 3},
  KeyM: {"PlayCard": 4},
};
const direction_keys = {
  KeyW: ["vertical", -1],
  ArrowUp: ["vertical", -1],
  KeyA: ["horizontal", -1],
  ArrowLeft: ["horizontal", -1],
  KeyS: ["vertical", 1],
  ArrowDown: ["vertical", 1],
  KeyD: ["horizontal", 1],
  ArrowRight: ["horizontal", 1],
};

let dpr = null;
let canvas_css_size = null;
let canvas_physical_size = null;
let resized = true;
window.addEventListener("resize", () => {resized = true;});
const update_canvas_size = () => {
  if (resized || dpr !== (window.devicePixelRatio || 1.0)) {
    resized = false;
    dpr = window.devicePixelRatio || 1.0;
    canvas_css_size = [window.innerWidth, window.innerHeight];
    [canvas.width, canvas.height] = canvas_physical_size = canvas_css_size.map(d => d*dpr);
    [canvas.style.width, canvas.style.height] = canvas_css_size.map(d => d+"px");
  }
}

window.clear_canvas = () => {
  context.fillStyle = "black";
  context.fillRect(0, 0, context.canvas.width, context.canvas.height);
};

window.draw_rect = (cx, cy, sx, sy, color) => {
  context.save();
  context.translate (cx, cy);
  context.fillStyle = color;
  context.fillRect(-sx/2.0,-sy/2.0, sx,sy);
  context.restore();
};

window.debug = message => {
  document.getElementById("debug").textContent += message;
}

const movement_intents = {
  horizontal: [0],
  vertical: [0],
};
let action_intents = [];
document.body.addEventListener("keydown", (event) => {
  const key = event.code;
  if (direction_keys[key] !== undefined) {
    event.preventDefault();
    const [dimension, direction] = direction_keys[key];
    if (!movement_intents[dimension].includes(direction)) {
      // in movement, the latest is given priority (pressing right overrides left)
      movement_intents[dimension].unshift(direction);
    }
  }
  if (action_keys[key] !== undefined) {
    event.preventDefault();
    if (!action_intents.includes(key)) {
      // for actions, the first is given priority (you can't interrupt yourself)
      action_intents.push(key);
    }
  }
});
document.body.addEventListener("keyup", (event) => {
  const key = event.code;
  if (direction_keys[key] !== undefined) {
    event.preventDefault();
    const [dimension, direction] = direction_keys[key];
    movement_intents[dimension] = movement_intents[dimension].filter(i => i !== direction);
  }
  if (action_keys[key] !== undefined) {
    event.preventDefault();
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
    document.getElementById("debug").textContent = JSON.stringify(action_intents);

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
