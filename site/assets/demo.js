// Types the demo note line by line. The finished note is already in the HTML, so the page
// reads the same without JavaScript or with reduced motion; this only replays it.
(() => {
  "use strict";

  const note = document.querySelector("[data-demo]");
  const windowEl = document.querySelector("[data-demo-window]");
  const replay = document.querySelector("[data-replay]");
  const themeInputs = document.querySelectorAll('input[name="theme"]');
  if (!note || !windowEl) return;

  // Theme picker: repaint the window with one of the app's themes.
  themeInputs.forEach((input) => {
    input.addEventListener("change", () => {
      if (input.value === "system") windowEl.removeAttribute("data-theme");
      else windowEl.setAttribute("data-theme", input.value);
    });
  });

  const motion = window.matchMedia("(prefers-reduced-motion: reduce)");
  if (motion.matches) {
    if (replay) replay.hidden = true;
    return;
  }

  // Each row: the tokens of its input (class + text) and its result element.
  const rows = [...note.querySelectorAll(".row")].map((row) => {
    const dt = row.querySelector("dt");
    const dd = row.querySelector("dd");
    const tokens = dt
      ? [...dt.childNodes].map((node) => ({
          cls: node.nodeType === Node.ELEMENT_NODE ? node.className : "",
          text: node.textContent,
        }))
      : [];
    return { dt, dd, tokens };
  });

  const caret = document.createElement("span");
  caret.className = "caret";
  caret.setAttribute("aria-hidden", "true");

  let run = 0;
  const wait = (ms, id) =>
    new Promise((resolve, reject) =>
      setTimeout(() => (id === run ? resolve() : reject(new Error("cancelled"))), ms)
    );

  function clear() {
    for (const { dt, dd } of rows) {
      if (dt) dt.textContent = "";
      if (dd) {
        dd.classList.remove("shown");
        dd.classList.add("pending");
      }
    }
  }

  function finish() {
    caret.remove();
    for (const { dt, dd, tokens } of rows) {
      if (dt) {
        dt.textContent = "";
        for (const token of tokens) dt.append(piece(token.cls, token.text));
      }
      if (dd) dd.classList.remove("pending");
    }
  }

  function piece(cls, text) {
    if (!cls) return document.createTextNode(text);
    const span = document.createElement("span");
    span.className = cls;
    span.textContent = text;
    return span;
  }

  async function type() {
    const id = ++run;
    note.setAttribute("aria-busy", "true");
    clear();
    try {
      await wait(500, id);
      for (const { dt, dd, tokens } of rows) {
        if (!dt) {
          await wait(260, id);
          continue;
        }
        dt.append(caret);
        for (const token of tokens) {
          const target = piece(token.cls, "");
          dt.insertBefore(target, caret);
          for (const char of token.text) {
            target.textContent += char;
            await wait(char === " " ? 40 : 26 + Math.random() * 46, id);
          }
        }
        if (dd && dd.textContent) {
          await wait(90, id);
          dd.classList.add("shown");
          dd.classList.remove("pending");
        }
        await wait(dt.textContent.startsWith("#") ? 380 : 520, id);
      }
      caret.remove();
    } catch {
      return; // a newer run took over
    } finally {
      if (id === run) note.removeAttribute("aria-busy");
    }
  }

  if (replay) {
    replay.hidden = false;
    replay.addEventListener("click", () => type());
  }

  // Start typing the first time the window scrolls into view.
  if ("IntersectionObserver" in window) {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          observer.disconnect();
          type();
        }
      },
      { threshold: 0.35 }
    );
    observer.observe(windowEl);
  }

  motion.addEventListener?.("change", () => {
    if (motion.matches) {
      run++;
      finish();
    }
  });
})();
