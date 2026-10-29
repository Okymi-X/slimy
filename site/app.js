// Theme, still images and copy buttons. Everything degrades to the plain page without this file.
(function () {
  var root = document.documentElement;
  function store(k, v) { try { if (v === null) localStorage.removeItem(k); else localStorage.setItem(k, v); } catch (e) {} }
  function load(k) { try { return localStorage.getItem(k); } catch (e) { return null; } }

  // Theme
  var themeBtn = document.getElementById('theme');
  function isDark() {
    var t = root.getAttribute('data-theme');
    if (t) return t === 'dark';
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  }
  function paintTheme() {
    if (!themeBtn) return;
    themeBtn.setAttribute('aria-label', isDark() ? themeBtn.dataset.toLight : themeBtn.dataset.toDark);
  }
  if (themeBtn) {
    paintTheme();
    themeBtn.addEventListener('click', function () {
      var next = isDark() ? 'light' : 'dark';
      root.setAttribute('data-theme', next);
      store('slimy-theme', next);
      paintTheme();
    });
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', paintTheme);
  }

  // Still images: swap each animated picture to its poster.
  var motionBtn = document.getElementById('motion');
  var sources = Array.prototype.slice.call(document.querySelectorAll('source[data-still]'));
  var reduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  function still() {
    var p = load('slimy-motion');
    if (p === 'still') return true;
    if (p === 'play') return false;
    return reduce.matches;
  }
  function paintMotion() {
    var s = still();
    sources.forEach(function (el) { el.media = s ? 'all' : 'not all'; });
    if (motionBtn) motionBtn.setAttribute('aria-pressed', s ? 'true' : 'false');
  }
  paintMotion();
  if (motionBtn) {
    motionBtn.addEventListener('click', function () {
      store('slimy-motion', still() ? 'play' : 'still');
      paintMotion();
    });
  }
  reduce.addEventListener('change', paintMotion);

  // Copy buttons
  Array.prototype.forEach.call(document.querySelectorAll('[data-copy]'), function (b) {
    b.addEventListener('click', function () {
      var pre = document.getElementById(b.getAttribute('data-copy'));
      if (!pre || !navigator.clipboard) return;
      navigator.clipboard.writeText(pre.innerText.trim()).then(function () {
        var old = b.textContent;
        b.textContent = b.dataset.done || 'Copied';
        setTimeout(function () { b.textContent = old; }, 1600);
      }, function () {});
    });
  });
})();
