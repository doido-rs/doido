// User-selectable theme: Light / Dark / System.
//
// The *preference* (light | dark | auto) is stored in localStorage; the
// *resolved* theme (light | dark) is applied as data-theme on <html>. The
// initial data-theme is set before paint by the inline script in
// partials/head.html to avoid a flash; this script keeps the selector's active
// state in sync, reacts to clicks, and follows the OS while in "auto".
(function () {
  var STORAGE_KEY = "theme";
  var root = document.documentElement;
  var mql = window.matchMedia("(prefers-color-scheme: dark)");

  function stored() {
    try { return localStorage.getItem(STORAGE_KEY); } catch (e) { return null; }
  }
  function preference() {
    return stored() || root.getAttribute("data-theme-default") || "auto";
  }
  function resolve(pref) {
    return pref === "light" || pref === "dark" ? pref : (mql.matches ? "dark" : "light");
  }
  function apply(pref) {
    root.dataset.theme = resolve(pref);
    var opts = document.querySelectorAll(".theme-option");
    for (var i = 0; i < opts.length; i++) {
      var active = opts[i].getAttribute("data-theme-value") === pref;
      opts[i].classList.toggle("active", active);
      opts[i].setAttribute("aria-pressed", active ? "true" : "false");
    }
  }

  // Align the selector with the stored preference on load.
  apply(preference());

  // Select a theme.
  document.addEventListener("click", function (e) {
    var btn = e.target && e.target.closest ? e.target.closest(".theme-option") : null;
    if (!btn) return;
    var pref = btn.getAttribute("data-theme-value");
    try { localStorage.setItem(STORAGE_KEY, pref); } catch (e2) {}
    apply(pref);
  });

  // Follow the OS while the user's preference is "auto".
  var onSystemChange = function () { if (preference() === "auto") apply("auto"); };
  if (mql.addEventListener) { mql.addEventListener("change", onSystemChange); }
  else if (mql.addListener) { mql.addListener(onSystemChange); }
})();
