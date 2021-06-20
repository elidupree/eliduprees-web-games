"use strict";

window.devicePixelRatio = window.devicePixelRatio || 1.0;
window.eliduprees_web_games = {
  cancel_starting: false,
};
window.auto_constants = {};
window.turn = Math.PI*2;

function auto_constant(name, default_value) {
  if (auto_constants[name] === undefined) {
    auto_constants[name] = default_value;
  }
  return auto_constants[name];
}

// Work around a platform-dependent issue
// https://stackoverflow.com/questions/39000273/iphone-landscape-scrolls-even-on-empty-page
if (document.documentElement.classList.contains("whole_window") && /iPhone|iPod/.test(navigator.userAgent)) {
  document.body.addEventListener ("touchmove", function(event) {event.preventDefault();});
  window.scrollTo (0,0);
}

window.eliduprees_web_games.fade_children = function (element, progress) {
      var children = element.children();
      var length = children.length ;;
      for (var index = 0; index <length ;++index) {
        var begin = index/length;
        var end = (index + 1)/length;
        var adjusted = Math.max (0, Math.min (1, (progress - begin)/(end - begin)));
        var pointer_events = "auto";
        if (adjusted < 0.1) { pointer_events = "none"; }
        children.eq(index).css ({opacity: adjusted, "pointer-events": pointer_events });
      }
    }


window.eliduprees_web_games.update_auto_constants_editor = () => {
  const container = document.getElementById("auto_constants_editor");
  for (const [name, value] of Object.entries(window.auto_constants)) {
    const id = `auto_constants_editor_${name}`;
    if (!document.getElementById(id)) {
      const new_container = document.createElement("div");
      new_container.id = id;
      new_container.className = "auto_constants_editor_entry";
      if (typeof value === "boolean") {
        const checkbox = document.createElement("input");
        checkbox.id = `${id}_checkbox`;
        checkbox.setAttribute("type", "checkbox");
        checkbox.checked = value;

        const label = document.createElement("label");
        label.setAttribute("for", checkbox.id);
        label.textContent = name;

        new_container.appendChild(checkbox);
        new_container.appendChild(label);
      } else if (typeof value === "number") {
        const number_input = document.createElement("input");
        const range_input = document.createElement("input");

        const update_range_input = val => {
          range_input.value = val;
          if (val > 0) {
            range_input.setAttribute("min", val * 0.01);
            range_input.setAttribute("max", val * 5);
          } else {
            range_input.setAttribute("min", val * 5);
            range_input.setAttribute("max", -val * 5);
          }
          range_input.setAttribute("step", val * 0.01);
        }

        number_input.id = `${id}_number`;
        number_input.setAttribute("type", "number");
        number_input.value = value;
        number_input.addEventListener("input", (event) => {
          auto_constants[name] = number_input.valueAsNumber;
          update_range_input(number_input.valueAsNumber);
        });

        range_input.id = `${id}_range`;
        range_input.setAttribute("type", "range");
        update_range_input(value);
        range_input.addEventListener("input", (event) => {
          auto_constants[name] = range_input.valueAsNumber;
          number_input.value = range_input.valueAsNumber;
        });

        const label = document.createElement("label");
        label.setAttribute("for", number_input.id);
        label.textContent = name;

        new_container.appendChild(range_input);
        new_container.appendChild(number_input);
        new_container.appendChild(label);
      }
      container.appendChild(new_container);
    }
  }
};
