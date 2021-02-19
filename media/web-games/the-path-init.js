"use strict";

if (!window.Path2D) {
  $(".loading").removeClass("loading").text ("(This game does not work in your browser. It should work in current versions of Chrome, Edge, Firefox and Safari.  Technical details: Path2D not supported.)");
  window.eliduprees_web_games.cancel_starting = true;
}
