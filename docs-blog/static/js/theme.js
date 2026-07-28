// User-selectable theme: Light or Dark only.
//
// Only "light" or "dark" is ever stored in localStorage. When nothing is stored
// yet, the theme is seeded from `data-theme-default` (or the OS when that is
// "auto"), and the OS is followed live until the user makes an explicit choice.
// The initial data-theme is set before paint by the inline script in
// partials/head.html; this keeps the selector's active state in sync and reacts
// to clicks.
(function () {
  var STORAGE_KEY = "theme";
  var root = document.documentElement;
  var mql = window.matchMedia("(prefers-color-scheme: dark)");

  function stored() {
    try {
      return localStorage.getItem(STORAGE_KEY);
    } catch (e) {
      return null;
    }
  }
  function hasChoice() {
    var s = stored();
    return s === "light" || s === "dark";
  }
  // The theme currently shown (light | dark).
  function resolved() {
    if (hasChoice()) return stored();
    var def = root.getAttribute("data-theme-default") || "auto";
    if (def === "light" || def === "dark") return def;
    return mql.matches ? "dark" : "light";
  }
  function apply(theme) {
    root.dataset.theme = theme;
    var opts = document.querySelectorAll(".theme-option");
    for (var i = 0; i < opts.length; i++) {
      var active = opts[i].getAttribute("data-theme-value") === theme;
      opts[i].classList.toggle("active", active);
      opts[i].setAttribute("aria-pressed", active ? "true" : "false");
    }
  }

  // Highlight the button matching the current theme on load.
  apply(resolved());

  // Pick Light or Dark.
  document.addEventListener("click", function (e) {
    var btn = e.target && e.target.closest ? e.target.closest(".theme-option") : null;
    if (!btn) return;
    var choice = btn.getAttribute("data-theme-value"); // "light" | "dark"
    try {
      localStorage.setItem(STORAGE_KEY, choice);
    } catch (e2) {}
    apply(choice);
  });

  // Follow the OS until the user makes an explicit choice.
  var onSystemChange = function () {
    if (!hasChoice()) apply(resolved());
  };
  if (mql.addEventListener) { mql.addEventListener("change", onSystemChange); }
  else if (mql.addListener) { mql.addListener(onSystemChange); }
})();
