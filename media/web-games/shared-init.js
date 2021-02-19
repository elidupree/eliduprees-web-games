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
if ($("html").hasClass("whole_window") && /iPhone|iPod/.test(navigator.userAgent)) {
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
