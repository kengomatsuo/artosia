// Motion and the glow follow matsuokengo.com's own script
// (kengomatsuo/kengomatsuo at 8387491, script.js): the staged entrance,
// the observer, and the button border that turns toward the pointer.

const REPO = "kengomatsuo/artosia";

// Remember a language the visitor chose, so the first page stops redirecting.
document.querySelectorAll("[data-lang]").forEach((link) => {
  link.addEventListener("click", () => {
    try {
      localStorage.setItem("lang", link.dataset.lang);
    } catch {}
  });
});

// The language menu closes on a press outside it and on Escape.
const languageMenu = document.querySelector(".lang-menu details");
if (languageMenu) {
  document.addEventListener("click", (event) => {
    if (!languageMenu.contains(event.target)) languageMenu.open = false;
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") languageMenu.open = false;
  });
}

window.addEventListener("load", () => {
  const loadTime = Date.now();
  const HERO_DONE = 550;

  const appearObserver = new IntersectionObserver(
    (entries) => {
      const toShow = [];
      entries.forEach(({ target, isIntersecting }) => {
        if (!isIntersecting) return;
        appearObserver.unobserve(target);
        toShow.push(target);
      });
      if (!toShow.length) return;
      const wait = Math.max(0, HERO_DONE - (Date.now() - loadTime));
      setTimeout(
        () =>
          requestAnimationFrame(() => {
            toShow.forEach((t) => t.classList.add("visible"));
          }),
        wait,
      );
    },
    { threshold: 0.08, rootMargin: "0px 0px -20px 0px" },
  );

  document.querySelectorAll(".hero-btn").forEach((btn, index) => {
    // +90 converts JS atan2 (0=right) to CSS angle convention (0=top)
    function cornerAngle() {
      const r = btn.getBoundingClientRect();
      return (
        ((Math.atan2(-r.height / 2, -r.width / 2) * 180) / Math.PI + 90 + 360) %
        360
      );
    }

    let home = cornerAngle();
    let current = home - 90,
      target = home;
    let intro = false;
    let rafId = null;

    btn.style.setProperty("--card-light-angle", current + "deg");
    setTimeout(
      () => {
        btn.style.setProperty("--card-glow", "1");
        intro = true;
        if (!rafId) rafId = requestAnimationFrame(tick);
      },
      100 + index * 250,
    );

    function tick() {
      const lerpFactor = intro ? 0.04 : 0.18;
      const diff = target - current;
      if (Math.abs(diff) > 0.2) current += diff * lerpFactor;
      else {
        current = target;
        intro = false;
      }
      btn.style.setProperty("--card-light-angle", current + "deg");
      if (Math.abs(diff) > 0.2) rafId = requestAnimationFrame(tick);
      else rafId = null;
    }

    btn.addEventListener("mousemove", (e) => {
      const r = btn.getBoundingClientRect();
      const relX = (e.clientX - r.left) / r.width;
      const rightAngle =
        ((Math.atan2(-r.height / 2, r.width / 2) * 180) / Math.PI + 90 + 360) %
        360;
      const span = ((rightAngle - home + 180 + 360) % 360) - 180;
      const raw = home + relX * span;
      const delta = ((((raw - target + 180) % 360) + 360) % 360) - 180;
      target += delta;
      if (!rafId) rafId = requestAnimationFrame(tick);
    });

    btn.addEventListener("mouseleave", () => {
      home = cornerAngle();
      const delta = ((((home - target + 180) % 360) + 360) % 360) - 180;
      target += delta;
      if (!rafId) rafId = requestAnimationFrame(tick);
    });
  });

  // Wait for first paint before triggering appear animations
  requestAnimationFrame(() =>
    setTimeout(() => {
      const show = (selector, delay) =>
        setTimeout(
          () => document.querySelector(selector)?.classList.add("visible"),
          delay,
        );
      // Nav first, then the hero rises piece by piece
      show("nav", 0);
      show("#hero h1", 60);
      show(".hero-sub", 200);
      show(".hero-actions", 310);
      show("#stage", 420);

      document
        .querySelectorAll("main .appear:not(#hero .appear), footer.appear")
        .forEach((el) => appearObserver.observe(el));

      const nav = document.querySelector("nav");
      const onScroll = () => nav.classList.toggle("scrolled", window.scrollY > 60);
      window.addEventListener("scroll", onScroll, { passive: true });
      onScroll();
    }, 100),
  );

  stage();
  release();
});

// The window on the page: Prev and Next step through real captures.
function stage() {
  const root = document.getElementById("stage");
  if (!root) return;
  const frames = Array.from(root.querySelectorAll(".stage-frames picture"));
  const count = root.querySelector(".stage-count");
  const [prev, next] = root.querySelectorAll("[data-step]");
  let at = 0;

  // The later frames are fetched once the first has drawn.
  frames.slice(1).forEach((picture) => {
    picture.querySelectorAll("[data-srcset]").forEach((s) => (s.srcset = s.dataset.srcset));
    picture.querySelectorAll("[data-src]").forEach((i) => (i.src = i.dataset.src));
  });

  function go(to) {
    at = Math.max(0, Math.min(frames.length - 1, to));
    frames.forEach((picture, i) => picture.classList.toggle("current", i === at));
    prev.disabled = at === 0;
    next.disabled = at === frames.length - 1;
    count.textContent = count.dataset.count
      .replace("{n}", at + 1)
      .replace("{of}", frames.length);
  }

  prev.addEventListener("click", () => go(at - 1));
  next.addEventListener("click", () => go(at + 1));
  root.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown" || e.key === "ArrowRight") go(at + 1);
    else if (e.key === "ArrowUp" || e.key === "ArrowLeft") go(at - 1);
    else return;
    e.preventDefault();
    (next.disabled ? prev : e.key === "ArrowUp" || e.key === "ArrowLeft" ? prev : next).focus();
  });
  go(0);
}

// The page is built with the version and size of its day; this brings them up
// to the newest release, kept for an hour so the API's limit is never met.
async function release() {
  const versions = document.querySelectorAll("[data-version]");
  const sizes = document.querySelectorAll("[data-size]");
  const zipSizes = document.querySelectorAll("[data-zip-size]");
  const linuxSizes = document.querySelectorAll("[data-linux-size]");
  const macSizes = document.querySelectorAll("[data-mac-size]");
  if (!versions.length) return;
  const HOUR = 3600e3;
  let found = null;
  try {
    const kept = JSON.parse(localStorage.getItem("release") || "null");
    if (kept && kept.zip && kept.linux !== undefined && Date.now() - kept.at < HOUR) found = kept;
  } catch {}
  if (!found) {
    try {
      const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
        headers: { Accept: "application/vnd.github+json" },
      });
      if (!res.ok) return;
      const latest = await res.json();
      const file = (name) => (latest.assets || []).find((a) => a.name === name);
      const setup = file("Artosia-setup.exe");
      const zip = file("Artosia-windows.zip");
      const linux = file("Artosia-linux-x86_64.tar.gz");
      const mac = file("Artosia-macos-aarch64.dmg");
      if (!setup || !zip || !latest.tag_name) return;
      const megabytes = (bytes) => Math.max(1, Math.round(bytes / 1048576));
      found = {
        at: Date.now(),
        version: latest.tag_name.replace(/^v/, ""),
        mb: megabytes(setup.size),
        zip: megabytes(zip.size),
        linux: linux ? megabytes(linux.size) : 0,
        mac: mac ? megabytes(mac.size) : 0,
      };
      try {
        localStorage.setItem("release", JSON.stringify(found));
      } catch {}
    } catch {
      return; // offline or limited: the built-in numbers stand
    }
  }
  versions.forEach((el) => (el.textContent = found.version));
  sizes.forEach((el) => (el.textContent = found.mb));
  zipSizes.forEach((el) => (el.textContent = found.zip));
  // A release without the file keeps the number the page was built with.
  if (found.linux) linuxSizes.forEach((el) => (el.textContent = found.linux));
  if (found.mac) macSizes.forEach((el) => (el.textContent = found.mac));
}

// The visitor's own system comes first, with the filled button, and the hero offers its file.
// Without this script Windows stays first and every system stays reachable. ?os=linux or
// ?os=mac overrides the guess, which is how the page is checked.
function system() {
  const rows = [...document.querySelectorAll(".os-row")];
  const raw = (
    new URLSearchParams(location.search).get("os") ||
    (navigator.userAgentData && navigator.userAgentData.platform) ||
    navigator.platform ||
    ""
  ).toLowerCase();
  const android = /android/i.test(navigator.userAgent);
  // A phone or a tablet runs none of the three, so no row is its own there: the three buttons
  // look alike and keep their order. An iPad says it is a Mac and has a touch screen.
  const handheld =
    !new URLSearchParams(location.search).get("os") &&
    (android || /iphone|ipad|ipod/i.test(navigator.userAgent) || (/^mac/.test(raw) && navigator.maxTouchPoints > 1));
  const guess = /^mac/.test(raw) ? "mac" : /linux/.test(raw) && !android ? "linux" : "windows";
  // A system with no file on the page (the Mac, while it has none) is shown as Windows.
  const mine = !handheld && rows.some((row) => row.dataset.os === guess) ? guess : "windows";
  document.documentElement.dataset.os = mine;
  rows.forEach((row) => {
    const own = !handheld && row.dataset.os === mine;
    row.classList.toggle("is-mine", own);
    // One button for every system, the same size; the visitor's own is the filled one.
    const link = row.querySelector(".dl");
    link.classList.toggle("hero-btn--primary", own);
    link.classList.toggle("hero-btn--quiet", !own);
  });
  const row = document.querySelector(`.os-row[data-os="${mine}"]`);
  const hero = document.querySelector(".dl-hero");
  hero.innerHTML = row.querySelector(".dl").innerHTML;
  // Every button that only says Download gives the visitor's own file.
  document.querySelectorAll(".dl-own, .dl-hero").forEach((link) => (link.href = row.querySelector(".dl").href));
  document.querySelector("#hero .release-note").innerHTML = row.querySelector(".os-file").innerHTML;
  document.querySelectorAll(".also-line").forEach((line) => (line.hidden = line.dataset.for !== mine));
}

system();
