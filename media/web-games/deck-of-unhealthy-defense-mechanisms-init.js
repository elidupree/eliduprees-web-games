"use strict";

import init, { rust_init, rust_do_frame, }
  from '/deck-of-unhealthy-defense-mechanisms/pkg/deck_of_unhealthy_defense_mechanisms.js';

const canvas = document.getElementById("canvas");
const context = canvas.getContext('2d');

const action_keys = {
  KeyZ: "PlayCard",
  KeyX: "ActivateMechanism",
};
const rotate_keys = {
  KeyX: -1,
  KeyV: 1,
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
let card_rotations_since_last_frame = 0;
let initiated_interaction = null;
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
  context.fillStyle = "#000";
  context.fillRect(0, 0, context.canvas.width, context.canvas.height);
};

window.panicked = () => {
  window.eliduprees_web_games.panicked = true;
}

window.draw_rect = (cx, cy, sx, sy, color) => {
  context.save();
  context.translate (cx, cy);
  context.fillStyle = color;
  context.fillRect(-sx/2.0,-sy/2.0, sx,sy);
  context.restore();
};

window.draw_text = (x, y, size, color, text) => {
  context.fillStyle = color;
  context.font = (size * dpr)+"px sans-serif";
  context.fillText(text, x, y);
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
      if (initiated_interaction === null) {
        initiated_interaction = action_keys[key];
      }
    }
  }
  if (rotate_keys[key] !== undefined) {
    event.preventDefault();
    card_rotations_since_last_frame += rotate_keys[key];
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
    let ongoing_intent;
    if (action_intents.length === 0) {
      ongoing_intent = {"Move": [movement_intents.horizontal[0], movement_intents.vertical[0]]};
    } else {
      ongoing_intent = {"Interact": action_keys[action_intents[0]]};
    }
    document.getElementById("debug").textContent = JSON.stringify(action_intents);

    update_canvas_size();
    eliduprees_web_games.update_auto_constants_editor();
    if (!window.eliduprees_web_games.panicked) {
      rust_do_frame(time, {
        ongoing_intent,
        card_rotations_since_last_frame,
        initiated_interaction,
        canvas_css_size,
        canvas_physical_size,
      });
    }
    card_rotations_since_last_frame = 0;
    initiated_interaction = null;
  }
  window.requestAnimationFrame(frame);
}

run();
