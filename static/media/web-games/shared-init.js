"use strict";

window.eliduprees_web_games = {
  cancel_starting: false,
};
window.auto_constants = {};

// Work around a platform-dependent issue
// https://stackoverflow.com/questions/39000273/iphone-landscape-scrolls-even-on-empty-page
if ($("html").hasClass("whole_window") && /iPhone|iPod/.test(navigator.userAgent)) {
  document.body.addEventListener ("touchmove", function(event) {event.preventDefault();});
  window.scrollTo (0,0);
}
