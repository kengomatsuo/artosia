"use strict";
/* The remote's script API for the look pages: the same pairing, long poll, commands, language, clock,
   keys and rules about what the room sees as src/page.html, with none of its markup. A look page owns its
   HTML and CSS and draws through the fill* helpers below; this file never decides how anything looks.
   The page names the element it wants by these ids and attributes:
     #pair #pin #pair-btn #pair-error ; #remote ; #lost
     [data-i] [data-i-ph] [data-i-aria] [data-lang-switch]
     [data-act="prev|next|plan-show"] ; [data-toggle="text|black|logo|live"] ; #logo-note
     #search ; #msg #msg-show #msg-hide #msg-state ; #minutes #cd-start #cd-stop ; [data-clock] [data-clock-pill]
   Strings are English only; the language switch keeps the choice and falls back to English. */
(() => {
const STR = {
  en: {
    pair_title: "Artosia remote", pin: "PIN", connect: "Connect", pin_hint: "Type the PIN shown on the computer",
    wrong_pin: "Wrong PIN", too_many: "Too many tries. Wait a minute.", unreachable: "Cannot reach the computer",
    tab_live: "Live", tab_plan: "Plan", tab_search: "Search", tab_more: "More", settings: "Settings", back_to_live: "Back to live",
    live: "Live", next: "Next", next_item: "Next item", next_btn: "Next", back: "Back", next_up: "Next up", on_air: "On air", off_air: "Off air",
    nothing_open: "Nothing open", nothing_live: "Nothing live",
    n_of: "{0} of {1}", n_slides: "{0} slides",
    clear_text: "Clear text", black: "Black", logo: "Logo",
    logo_needs: "Set a logo picture in Artosia first",
    text_cleared: "Text is cleared", screen_black: "Screen is black", logo_showing: "Logo is showing",
    projector_off: "Projector is off",
    jump: "Jump to a section", sections: "Sections", output: "Output",
    plan_pick: "Pick an item", plan_show: "Show {0}", plan_live: "Live", plan_empty: "The plan is empty",
    search_ph: "Search songs and verses", search_hint: "Type a title, a line or a verse like John 3 16",
    searching: "Searching", no_results: "No results",
    show: "Show", add: "Add", added: "Added", show_aria: "Show {0} now", add_aria: "Add {0} to the plan", cancel: "Not now",
    message: "Message to the room", msg_tab: "Message", screen: "Screen", message_ph: "Write a message", message_show: "Show", message_hide: "Hide",
    message_showing: "Showing: {0}",
    countdown: "Countdown", minutes: "Minutes", start: "Start", stop: "Stop",
    language: "Language", slides_grid: "Change the columns",
    keys_title: "Keys",
    keys_text: "Space or the down arrow: next. Up arrow: back. / : search. C: chorus. V: next verse. B: bridge. 1 to 9: that section. F1 no text, F2 black, F3 logo, F6 projector.",
    lost: "Lost connection", retrying: "Trying again"
  }
};
const LANGS = [["en", "English"], ["id", "Bahasa Indonesia"]];
const KEY = "artosia-remote-token";
const LANG_KEY = "artosia-remote-lang";
const $ = (id) => document.getElementById(id);
const $$ = (sel) => document.querySelectorAll(sel);

const R = {
  state: null, lang: "en", lost: false, picked: null, token: null,
  /* A look page sets what it needs: render(state, view), lang(), lost(bool), openSearch(), openLive(). */
  hooks: {},
};
window.Remote = R;
let version = 0, polling = false, clockOffset = 0, searchTimer = null, sentSearch = "", msgDirty = false, clockTimer = null;

function store(name, get, value) {
  try {
    if (get) return localStorage.getItem(name);
    if (value) localStorage.setItem(name, value); else localStorage.removeItem(name);
  } catch (e) { /* storage blocked: the choice lasts for this page only */ }
  return null;
}
R.t = (key, ...args) => {
  let s = (STR[R.lang] && STR[R.lang][key]) || STR.en[key] || key;
  args.forEach((a, i) => { s = s.replace("{" + i + "}", a); });
  return s;
};
const t = R.t;
R.nice = (label) => label;
R.$ = $;
R.el = (tag, cls, text) => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
};
R.icon = (name) => {
  const ns = document.querySelector("svg").namespaceURI;
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("class", "ic");
  svg.setAttribute("aria-hidden", "true");
  const use = document.createElementNS(ns, "use");
  use.setAttribute("href", "#i-" + name);
  svg.append(use);
  return svg;
};
const cache = {};
R.changed = (key, value) => {
  const sig = JSON.stringify(value);
  if (cache[key] === sig) return false;
  cache[key] = sig;
  return true;
};
R.fmt = (sec) => Math.floor(sec / 60) + ":" + String(sec % 60).padStart(2, "0");
R.groupClass = (group) => " g" + Math.min(Math.max(group | 0, 0), 5);
R.kindIcon = (kind) => (["song", "passage", "announcements"].includes(kind) ? kind : "item");

/* ---------- language ---------- */
function pickLang() {
  const saved = store(LANG_KEY, true);
  return LANGS.some(([c]) => c === saved) ? saved : "en";
}
function applyLang() {
  document.documentElement.lang = R.lang;
  $$("[data-i]").forEach((e) => { e.textContent = t(e.dataset.i); });
  $$("[data-i-ph]").forEach((e) => { e.placeholder = t(e.dataset.iPh); });
  $$("[data-i-aria]").forEach((e) => { e.setAttribute("aria-label", t(e.dataset.iAria)); });
  $$("[data-lang-switch]").forEach((box) => {
    box.replaceChildren();
    for (const [code, name] of LANGS) {
      const b = R.el("button", "seg-btn press" + (R.lang === code ? " on" : ""), name);
      b.type = "button"; b.dataset.keep = "";
      b.setAttribute("aria-pressed", String(R.lang === code));
      b.addEventListener("click", () => { R.lang = code; store(LANG_KEY, false, code); applyLang(); });
      box.append(b);
    }
  });
  for (const k of Object.keys(cache)) delete cache[k];
  if (R.hooks.lang) R.hooks.lang();
  if (R.state) render();
  refreshPlanShow(); refreshSearchHint(); tickClock();
}

/* ---------- pairing, polling, commands ---------- */
function show(which) {
  $("pair").hidden = which !== "pair";
  $("remote").hidden = which !== "remote";
  if (which !== "remote") setLost(false);
}
function setLost(v) {
  R.lost = v;
  $("lost").hidden = !v;
  $$("#remote button:not([data-keep])").forEach((b) => {
    if (v) { b.dataset.wasDisabled = b.disabled ? "1" : ""; b.disabled = true; }
    else if (b.dataset.wasDisabled !== undefined) { b.disabled = b.dataset.wasDisabled === "1"; delete b.dataset.wasDisabled; }
  });
  for (const id of ["search", "msg", "minutes"]) if ($(id)) $(id).disabled = v;
  if (R.hooks.lost) R.hooks.lost(v);
  if (!v && R.state) render();
}
async function pair(pin) {
  $("pair-error").textContent = "";
  try {
    const r = await fetch("/api/pair", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ pin }) });
    if (r.ok) { R.token = (await r.json()).token; store(KEY, false, R.token); start(); return; }
    $("pair-error").textContent = r.status === 429 ? t("too_many") : t("wrong_pin");
  } catch (e) { $("pair-error").textContent = t("unreachable"); }
  show("pair");
  if (R.hooks.pinReset) R.hooks.pinReset();
}
function start() {
  show("remote");
  if (R.hooks.start) R.hooks.start();
  if (!polling) { polling = true; poll(); }
}
function unpair() {
  R.token = null; version = 0; R.state = null;
  store(KEY, false, null);
  $("pin").value = "";
  if (R.hooks.pinReset) R.hooks.pinReset();
  show("pair");
}
async function poll() {
  while (R.token) {
    const ctl = new AbortController();
    const timer = setTimeout(() => ctl.abort(), 30000);
    try {
      const r = await fetch("/api/state?since=" + version, { headers: { Authorization: "Bearer " + R.token }, signal: ctl.signal, cache: "no-store" });
      if (r.status === 401) { unpair(); break; }
      if (!r.ok) throw new Error("status " + r.status);
      const s = await r.json();
      if (R.lost) setLost(false);
      if (s.version !== version) {
        version = s.version; R.state = s;
        if (typeof s.now === "number") clockOffset = s.now - Date.now() / 1000;
        render();
      }
    } catch (e) {
      setLost(true);
      await new Promise((res) => setTimeout(res, 1500));
    } finally { clearTimeout(timer); }
  }
  polling = false;
}
async function send(cmd, quiet) {
  if (!R.token || R.lost) return;
  if (!quiet && navigator.vibrate) { try { navigator.vibrate(8); } catch (e) { /* ignore */ } }
  try {
    const r = await fetch("/api/command", { method: "POST", headers: { Authorization: "Bearer " + R.token, "Content-Type": "application/json" }, body: JSON.stringify(cmd) });
    if (r.status === 401) unpair();
    else if (!r.ok && r.status !== 400) setLost(true);
  } catch (e) { setLost(true); }
}
R.send = send;
R.setLost = setLost;

/* ---------- what the room sees ---------- */
R.view = (s) => {
  const n = s.slides.length, live = s.live;
  const cur = live !== null ? s.slides[live] : null;
  const nextIdx = live !== null ? live + 1 : (n ? 0 : -1);
  const nx = nextIdx >= 0 && nextIdx < n ? s.slides[nextIdx] : null;
  /* Red while the room sees these words; yellow for what keeps them from it. */
  const kept = !cur || s.off || s.hidden || s.black || s.logo;
  let at = -1;
  if (live !== null) s.sections.forEach((x, i) => { if (x.slide <= live) at = i; });
  return {
    title: s.title || t("nothing_open"),
    count: n && live !== null ? t("n_of", live + 1, n) : (n ? t("n_slides", n) : ""),
    n, live, cur, kept, onair: !kept,
    label: cur && cur.label ? R.nice(cur.label) : "",
    text: cur ? cur.text : t("nothing_live"),
    why: s.off ? t("projector_off") : (s.logo ? t("logo_showing") : (s.black ? t("screen_black") : (s.hidden ? t("text_cleared") : ""))),
    next: nx ? { tag: nx.label ? R.nice(nx.label) : t("next"), text: nx.text } : { tag: t("next_item"), text: s.next_title },
    hasNext: !!nx || !!s.next_title,
    at,
    progress: n && live !== null ? (live + 1) / n : 0,
    groupOf: (i) => { let g = null; s.sections.forEach((x) => { if (x.slide <= i) g = x.group; }); return g; },
  };
};

function render() {
  const s = R.state;
  const v = R.view(s);
  $$("[data-toggle]").forEach((b) => {
    const on = { text: s.hidden, black: s.black, logo: s.logo, live: !s.off }[b.dataset.toggle];
    b.classList.toggle("on", on); b.setAttribute("aria-pressed", String(on));
    if (b.dataset.toggle === "logo") b.disabled = !s.logo_ready || R.lost;
  });
  if ($("logo-note")) $("logo-note").hidden = s.logo_ready;
  if (R.picked !== null && !s.plan.some((p) => p.id === R.picked && p.kind !== "heading")) R.picked = null;
  if (R.hooks.render) R.hooks.render(s, v);
  markPicked(); refreshPlanShow();
  renderMessage(s);
  if (R.hooks.hits) R.hooks.hits(s, resultsReady(s));
  refreshSearchHint();
  tickClock();
  if (R.lost) setLost(true);
}

/* Each fill* draws a list once per change of its data, then only moves the marks. */
R.fillSections = (box, make) => {
  const s = R.state, v = R.view(s);
  box.hidden = !s.sections.length;
  if (R.changed("sections:" + box.id, [s.sections, R.lang])) {
    box.replaceChildren();
    s.sections.forEach((x) => {
      const b = make(x);
      b.addEventListener("click", () => send({ cmd: "goto", index: x.slide }));
      box.append(b);
    });
  }
  [...box.children].forEach((b, i) => {
    b.classList.toggle("live", i === v.at);
    if (i === v.at) b.setAttribute("aria-current", "true"); else b.removeAttribute("aria-current");
  });
  const chip = box.children[v.at];
  if (chip && R.changed("liveChip:" + box.id, [v.at, s.sections.length]) && box.scrollWidth > box.clientWidth) {
    box.scrollTo({ left: chip.offsetLeft - (box.clientWidth - chip.offsetWidth) / 2, behavior: "smooth" });
  }
};
R.fillSlides = (list, make) => {
  const s = R.state, live = s.live;
  if (R.changed("slides:" + list.id, [s.slides, s.sections, R.lang])) {
    list.replaceChildren();
    const v = R.view(s);
    s.slides.forEach((sl, i) => {
      const b = make(sl, i, v.groupOf(i));
      b.addEventListener("click", () => send({ cmd: "goto", index: i }));
      list.append(b);
    });
  }
  let liveEl = null;
  [...list.children].forEach((e, i) => {
    const on = i === live;
    e.classList.toggle("live", on);
    if (on) { e.setAttribute("aria-current", "true"); liveEl = e; } else e.removeAttribute("aria-current");
  });
  if (liveEl && R.changed("liveIdx:" + list.id, live)) liveEl.scrollIntoView({ block: "nearest", inline: "nearest" });
};
R.fillPlan = (list, makeHeading, makeRow) => {
  const s = R.state;
  if (R.changed("plan:" + list.id, [s.plan, R.lang])) {
    list.replaceChildren();
    if (!s.plan.length) list.append(R.el("p", "note", t("plan_empty")));
    let heading = -1;
    s.plan.forEach((p) => {
      if (p.kind === "heading") { heading += 1; list.append(makeHeading(p, heading)); return; }
      const b = makeRow(p, Math.max(heading, 0));
      b.dataset.id = p.id;
      b.addEventListener("click", () => { R.picked = R.picked === p.id ? null : p.id; markPicked(); refreshPlanShow(); });
      list.append(b);
    });
  }
  const liveId = (s.plan.find((p) => p.live) || {}).id;
  list.querySelectorAll("[data-id]").forEach((b) => {
    const on = String(liveId) === b.dataset.id;
    b.classList.toggle("live", on);
    if (on) b.setAttribute("aria-current", "true"); else b.removeAttribute("aria-current");
  });
};
function markPicked() {
  $$("[data-id]").forEach((b) => {
    const on = String(R.picked) === b.dataset.id;
    b.classList.toggle("picked", on);
    b.setAttribute("aria-pressed", String(on));
  });
}
function refreshPlanShow() {
  const p = R.state && R.picked !== null ? R.state.plan.find((x) => x.id === R.picked) : null;
  $$("[data-act=plan-show]").forEach((b) => { b.textContent = p ? t("plan_show", p.title) : t("plan_pick"); b.disabled = !p || R.lost; });
  $$("[data-picked]").forEach((e) => { e.hidden = !p; });
}
R.showPicked = () => {
  if (R.picked === null) return;
  send({ cmd: "plan", id: R.picked });
  R.picked = null; markPicked(); refreshPlanShow();
  if (R.hooks.openLive) R.hooks.openLive();
};
R.pick = (id) => { R.picked = R.picked === id ? null : id; markPicked(); refreshPlanShow(); };

/* ---------- search ---------- */
const typed = () => ($("search") ? $("search").value.trim() : "");
const resultsReady = (s) => typed() !== "" && s && s.hits_for.trim().toLowerCase() === typed().toLowerCase();
R.fillHits = (list, make) => {
  const s = R.state, ready = resultsReady(s);
  if (R.changed("hits:" + list.id, [ready ? s.hits : [], ready, R.lang])) {
    list.replaceChildren();
    if (ready) s.hits.forEach((h) => list.append(make(h)));
  }
};
/* The two buttons of a hit: Show opens the live screen, Add puts it on the plan and says so for a moment. */
R.hitButtons = (h, showCls, addCls) => {
  const show = R.el("button", showCls + " press", t("show"));
  show.type = "button"; show.setAttribute("aria-label", t("show_aria", h.title));
  show.addEventListener("click", (e) => { e.stopPropagation(); send({ cmd: "hit", id: h.id }); if (R.hooks.openLive) R.hooks.openLive(); });
  const add = R.el("button", addCls + " press", t("add"));
  add.type = "button"; add.setAttribute("aria-label", t("add_aria", h.title));
  add.addEventListener("click", (e) => {
    e.stopPropagation();
    send({ cmd: "add", id: h.id });
    add.textContent = t("added"); add.disabled = true;
    setTimeout(() => { add.textContent = t("add"); add.disabled = R.lost; }, 1500);
  });
  return [show, add];
};
function refreshSearchHint() {
  const e = $("search-hint");
  if (!e) return;
  const q = typed();
  e.textContent = !q ? t("search_hint") : (!resultsReady(R.state) ? t("searching") : (!R.state.hits.length ? t("no_results") : ""));
}

/* ---------- message and countdown ---------- */
function renderMessage(s) {
  if (!$("msg")) return;
  if ($("msg-state")) $("msg-state").textContent = s.message ? t("message_showing", s.message) : "";
  $("msg-hide").classList.toggle("on", !!s.message);
  $("msg-hide").disabled = !s.message || R.lost;
  if (R.changed("message", s.message)) msgDirty = false;
  if (!msgDirty && document.activeElement !== $("msg")) $("msg").value = s.message;
  refreshMsgShow();
}
function refreshMsgShow() { if ($("msg-show")) $("msg-show").disabled = !$("msg").value.trim() || R.lost; }
function secondsLeft() {
  const s = R.state;
  if (!s || s.countdown_ends_at === null || s.countdown_ends_at === undefined) return null;
  return Math.max(0, Math.ceil(s.countdown_ends_at - (Date.now() / 1000 + clockOffset)));
}
function tickClock() {
  const left = secondsLeft();
  $$("[data-clock-pill]").forEach((e) => { e.hidden = left === null; });
  $$("[data-clock]").forEach((e) => { e.textContent = left === null ? "" : R.fmt(left); });
  R.clockLeft = left;
  if ($("cd-stop")) $("cd-stop").disabled = left === null || R.lost;
  if (R.hooks.clock) R.hooks.clock(left);
  if (left !== null && !clockTimer) clockTimer = setInterval(tickClock, 250);
  if (left === null && clockTimer) { clearInterval(clockTimer); clockTimer = null; }
}
const minutesValue = () => {
  const m = parseInt($("minutes").value, 10);
  return Number.isFinite(m) && m >= 1 && m <= 180 ? m : null;
};

/* ---------- wiring ---------- */
function jumpTo(key) {
  const s = R.state;
  if (!s || !s.sections.length) return;
  const k = key === "r" ? "c" : key.toLowerCase();
  if (/^[0-9]$/.test(k)) {
    const nth = s.sections[k === "0" ? 9 : Number(k) - 1];
    if (nth) send({ cmd: "goto", index: nth.slide });
    return;
  }
  const hits = s.sections.filter((x) => (x.short || "").toLowerCase().startsWith(k));
  if (!hits.length) return;
  const here = s.live === null ? -1 : s.live;
  send({ cmd: "goto", index: (hits.find((x) => x.slide > here) || hits[0]).slide });
}
const next = () => send({ cmd: "next" });
const back = () => send({ cmd: "prev" });
function wire() {
  $$("[data-act=next]").forEach((b) => b.addEventListener("click", next));
  $$("[data-act=prev]").forEach((b) => b.addEventListener("click", back));
  $$("[data-act=plan-show]").forEach((b) => b.addEventListener("click", R.showPicked));
  $$("[data-toggle]").forEach((b) => b.addEventListener("click", () => send({ cmd: b.dataset.toggle })));
  if ($("msg")) {
    $("msg").addEventListener("input", () => { msgDirty = true; refreshMsgShow(); });
    $("msg").addEventListener("keydown", (e) => { if (e.key === "Enter" && !$("msg-show").disabled) $("msg-show").click(); });
    $("msg-show").addEventListener("click", () => { send({ cmd: "message", text: $("msg").value }); msgDirty = false; });
    $("msg-hide").addEventListener("click", () => { send({ cmd: "message", text: "" }); $("msg").value = ""; msgDirty = false; refreshMsgShow(); });
  }
  if ($("minutes")) {
    $("minutes").addEventListener("input", () => {
      const v = $("minutes").value.replace(/\D/g, "").slice(0, 3);
      if (v !== $("minutes").value) $("minutes").value = v;
      $("cd-start").disabled = minutesValue() === null || R.lost;
    });
    $("cd-start").addEventListener("click", () => { const m = minutesValue(); if (m !== null) send({ cmd: "countdown", minutes: m }); });
    $("cd-stop").addEventListener("click", () => send({ cmd: "countdown", minutes: 0 }));
  }
  if ($("search")) {
    $("search").addEventListener("input", () => {
      clearTimeout(searchTimer);
      refreshSearchHint();
      if (R.state) R.hooks.hits && R.hooks.hits(R.state, resultsReady(R.state));
      searchTimer = setTimeout(() => {
        const q = typed();
        if (q && q !== sentSearch) { sentSearch = q; send({ cmd: "search", text: q }, true); }
      }, 300);
    });
    $("search").addEventListener("keydown", (e) => {
      if (e.key === "Enter") { clearTimeout(searchTimer); const q = typed(); if (q) { sentSearch = q; send({ cmd: "search", text: q }, true); } e.target.blur(); }
    });
  }
  /* a mouse click must not leave focus on a button: Space would press it again */
  document.addEventListener("mousedown", (e) => { if (e.target.closest && e.target.closest("button")) e.preventDefault(); });
  document.addEventListener("keydown", (e) => {
    if (!R.token || $("remote").hidden || e.ctrlKey || e.metaKey || e.altKey) return;
    const tg = e.target;
    if (tg && (tg.tagName === "INPUT" || tg.tagName === "TEXTAREA" || tg.isContentEditable)) {
      if (e.key === "Escape") { tg.blur(); if (tg.id === "search" && R.hooks.openLive) R.hooks.openLive(); }
      return;
    }
    const k = e.key;
    let handled = true;
    if (k === " " || k === "ArrowDown" || k === "ArrowRight" || k === "PageDown") { if (!e.repeat) next(); }
    else if (k === "ArrowUp" || k === "ArrowLeft" || k === "PageUp") { if (!e.repeat) back(); }
    else if (k === "/") { if (R.hooks.openSearch) R.hooks.openSearch(); }
    else if (k === "F1") send({ cmd: "text" });
    else if (k === "F2") send({ cmd: "black" });
    else if (k === "F6") send({ cmd: "live" });
    else if (k === "F3") { if (R.state && R.state.logo_ready) send({ cmd: "logo" }); }
    else if (k.length === 1 && /[a-z0-9]/i.test(k) && !e.repeat) jumpTo(k);
    else handled = false;
    if (handled) e.preventDefault();
  });
  document.addEventListener("keyup", (e) => { if (e.key === " " && R.token && e.target && e.target.tagName === "BUTTON") e.preventDefault(); });
  document.addEventListener("visibilitychange", () => { /* the shipped page asks for a wake lock here; a look page does not */ });
  $("pair-btn").addEventListener("click", () => pair($("pin").value.trim()));
  $("pin").addEventListener("input", () => {
    const v = $("pin").value.replace(/\D/g, "").slice(0, 4);
    if (v !== $("pin").value) $("pin").value = v;
    if (R.hooks.pinInput) R.hooks.pinInput(v);
    if (v.length === 4) pair(v);
  });
  $("pin").addEventListener("keydown", (e) => { if (e.key === "Enter") pair($("pin").value.trim()); });
}

/* The page calls this once its markup and hooks exist. */
R.boot = () => {
  R.lang = pickLang();
  wire();
  applyLang();
  const q = new URLSearchParams(location.search).get("pin");
  const stay = new URLSearchParams(location.search).get("look");
  if (q) {
    /* the look stays in the address so a reload shows the same page */
    try { history.replaceState(null, "", location.pathname + (stay ? "?look=" + stay : "")); } catch (e) { /* ignore */ }
    pair(q.trim());
    return;
  }
  R.token = store(KEY, true);
  if (R.token) start(); else { show("pair"); $("pin").focus(); }
};
})();
