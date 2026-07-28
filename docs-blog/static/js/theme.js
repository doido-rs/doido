// Light/dark toggle. The initial theme is set before paint by the inline script
// in partials/head.html; this only handles the toggle button and persistence.
(function () {
  var btn = document.querySelector(".theme-toggle");
  if (!btn) return;

  btn.addEventListener("click", function () {
    var current = document.documentElement.dataset.theme === "dark" ? "dark" : "light";
    var next = current === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    try {
      localStorage.setItem("theme", next);
    } catch (e) {
      /* storage may be unavailable; the toggle still works for this session */
    }
  });
})();
