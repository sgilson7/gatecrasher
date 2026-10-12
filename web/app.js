// The page draws what the core returns and decides nothing. It loads the data
// files, starts the core through the shim, and each display frame passes the
// clock, the held actions and the mouse movement in, then draws what comes
// out. The core says which screen the game is on; the page shows it.
import init, { Game, numbers } from "./pkg/gatecrasher_shim.js";
import { makeRenderer } from "./gl.js";
import { makeInput } from "./input.js";
import { makeHud } from "./hud.js";
import { setup as setupSound, wake, toggleMute, play } from "./sound.js";

const BUILD = "__BUILD__";
const SAVE_KEY = "gatecrasher.save";
const el = (id) => document.getElementById(id);
const file = (path) => fetch(`${path}?v=${BUILD}`).then((r) => r.text());

// Browser storage can be missing or refused; the game works without it.
const storage = {
  load() {
    try {
      return localStorage.getItem(SAVE_KEY) || "";
    } catch {
      return "";
    }
  },
  save(text) {
    try {
      localStorage.setItem(SAVE_KEY, text);
    } catch {
      // nothing to do: the save is simply not kept
    }
  },
};

function fillCopy(copy) {
  for (const node of document.querySelectorAll("[data-copy]")) {
    node.textContent = copy[node.dataset.copy];
  }
  document.title = copy.title;
  el("build").textContent = BUILD;
  for (const table of document.querySelectorAll("table.controls")) {
    for (const [keys, does] of copy.controls) {
      const row = table.insertRow();
      row.insertCell().textContent = keys;
      row.insertCell().textContent = does;
    }
  }
}

function button(html, onClick) {
  const b = document.createElement("button");
  b.append(...html);
  b.addEventListener("click", onClick);
  return b;
}

function span(cls, text) {
  const s = document.createElement("span");
  s.className = cls;
  s.textContent = text;
  return s;
}

async function start() {
  const names = ["palette", "missions", "uniforms", "recruits", "talk", "copy", "controls", "sounds", "display"];
  const texts = await Promise.all(
    names.map((n) => file(`data/${n}.json`)).concat([file("data/shaders/scene.vert"), file("data/shaders/scene.frag")]),
  );
  const [palette, missions, uniforms, recruits, talk, copyText, controlsText, soundsText, displayText, vertex, fragment] = texts;
  const copy = JSON.parse(copyText);
  const display = JSON.parse(displayText);
  fillCopy(copy);
  setupSound(JSON.parse(soundsText));
  await init();
  const N = JSON.parse(numbers());

  let game;
  try {
    game = new Game(palette, missions, uniforms, recruits, talk, storage.load());
  } catch (e) {
    el("message").textContent = String(e);
    document.body.dataset.ready = "true";
    return;
  }

  const canvas = el("view");
  let renderer = null;
  try {
    renderer = makeRenderer(canvas, N, { vertex, fragment, software: display.software }, (id) => game.mesh(id));
  } catch (e) {
    el("message").textContent = String(e);
  }
  if (!renderer) el("message").textContent = el("message").textContent || copy.no_webgl;
  const save = () => storage.save(game.saved());
  const input = makeInput(canvas, JSON.parse(controlsText), N.actions, () => wake());
  const hud = makeHud(copy, display);

  // --- the menus, drawn from the core's menu JSON ---------------------------
  function drawMenus() {
    const m = JSON.parse(game.menu());
    el("mission-list").replaceChildren(
      ...m.missions.map((mi, k) =>
        button(
          [span("rec", mi.rank ? `${copy.best_rank} ${mi.rank} ${mi.time}` : copy.not_played), span("name", mi.name), document.createElement("br"), span("obj", mi.objective)],
          act(() => {
            game.choose(k);
            drawMenus();
          }),
        ),
      ),
    );
    const chosen = m.missions[m.chosen];
    el("b-art").src = `art/mission${m.chosen + 1}.png?v=${BUILD}`;
    el("b-name").textContent = chosen.name;
    el("b-brief").textContent = chosen.brief;
    el("b-objective").textContent = chosen.objective;
    const table = el("uniforms");
    table.replaceChildren();
    const head = table.insertRow();
    head.insertCell().textContent = "";
    for (const g of copy.grounds) head.insertCell().textContent = g;
    m.uniforms.forEach((u, k) => {
      const row = table.insertRow();
      row.className = k === m.uniform ? "on" : "";
      const name = row.insertCell();
      const sw = span("swatch", "");
      sw.style.background = `linear-gradient(135deg, ${u.base} 50%, ${u.blotch} 50%)`;
      name.append(sw, u.name);
      for (const v of [u.grass, u.dirt, u.concrete, u.floor, u.sand]) row.insertCell().textContent = v;
      row.addEventListener(
        "click",
        act(() => {
          game.set_uniform(k);
          save();
          drawMenus();
        }),
      );
    });
    el("g-staff").textContent = m.staff;
    const units = el("g-units");
    units.replaceChildren();
    const perks = [m.perks.sniff, m.perks.tranq, m.perks.battery];
    const perkNames = [copy.perk_intel, copy.perk_rnd, copy.perk_support];
    m.units.forEach((u, k) => {
      const row = units.insertRow();
      row.insertCell().textContent = u;
      row.insertCell().textContent = `${copy.unit_level} ${m.levels[k]}`;
      row.insertCell().textContent = `${perkNames[k]}: ${perks[k]}`;
    });
    const roster = el("g-roster");
    roster.replaceChildren(
      ...(m.roster.length
        ? m.roster.map(([name, unit, grade]) => {
            const li = document.createElement("li");
            li.textContent = `${name} (${unit}, ${grade})`;
            return li;
          })
        : [Object.assign(document.createElement("li"), { textContent: copy.roster_empty })]),
    );
  }

  function act(fn) {
    return () => {
      wake();
      fn();
      if (document.activeElement) document.activeElement.blur();
    };
  }
  const engage = () => input.engage(false);
  el("to-graceland").addEventListener("click", act(() => game.graceland()));
  el("brief-back").addEventListener("click", act(() => game.to_title()));
  el("deploy").addEventListener(
    "click",
    act(() => {
      save();
      game.deploy();
      engage();
    }),
  );
  el("engage-btn").addEventListener("click", act(engage));
  el("resume").addEventListener(
    "click",
    act(() => {
      game.resume();
      engage();
    }),
  );
  el("restart").addEventListener(
    "click",
    act(() => {
      game.retry();
      engage();
    }),
  );
  el("abort").addEventListener("click", act(() => game.to_title()));
  el("retry").addEventListener(
    "click",
    act(() => {
      game.retry();
      engage();
    }),
  );
  el("failed-title").addEventListener("click", act(() => game.to_title()));
  el("complete-title").addEventListener("click", act(() => game.to_title()));
  el("complete-graceland").addEventListener("click", act(() => game.graceland()));
  el("graceland-back").addEventListener("click", act(() => game.to_title()));

  // losing a pointer lock that was held pauses play
  let held = false;
  document.addEventListener("pointerlockchange", () => {
    const now = Boolean(document.pointerLockElement);
    if (held && !now && document.body.dataset.mode === "play") game.pause();
    held = now;
  });
  window.addEventListener("keydown", (e) => {
    const mode = document.body.dataset.mode;
    if (e.code === "Escape" && mode === "play") game.pause();
    if (e.code === "KeyN") toggleMute();
    if (e.code === "KeyI" && mode === "play") hud.toggleIntel();
    wake();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) game.pause();
  });

  function resize() {
    const scale = renderer && renderer.software ? display.software_scale : 1;
    const [w, h] = game.resize(canvas.clientWidth, canvas.clientHeight, (window.devicePixelRatio || 1) * scale);
    if (renderer) renderer.resize(w, h);
  }
  window.addEventListener("resize", resize);
  resize();

  const overlays = { title: "title", brief: "brief", paused: "paused", failed: "failed", complete: "complete", graceland: "graceland" };
  let shownMode = null;
  function showMode(h) {
    if (h.mode === shownMode) return;
    document.body.dataset.mode = h.mode;
    for (const [m, id] of Object.entries(overlays)) el(id).hidden = h.mode !== m;
    el("hud").hidden = !["play", "paused"].includes(h.mode);
    if (["title", "brief", "graceland", "complete"].includes(h.mode)) drawMenus();
    if (h.outcome) {
      const o = h.outcome;
      el("f-cause").textContent = copy[`cause_${o.cause}`] || "";
      el("f-mission").textContent = o.mission;
      el("c-mission").textContent = o.mission;
      el("c-rank").textContent = o.rank;
      el("c-time").textContent = o.time;
      el("c-alerts").textContent = o.alerts;
      el("c-kills").textContent = o.kills;
      el("c-kill-cost").textContent = copy.kill_cost_value.replace("{}", o.kill_cost);
      el("c-score").textContent = o.score;
      el("c-intel").textContent = `${o.intel} / ${o.intel_total}`;
      const items = o.recruits.length ? o.recruits : [copy.no_recruits];
      el("c-recruits").replaceChildren(
        ...items.map((t) => Object.assign(document.createElement("li"), { textContent: t })),
      );
    }
    if (h.mode === "complete" || h.mode === "failed") save();
    if (h.mode !== "play") input.release();
    shownMode = h.mode;
  }

  let sceneryVersion = null;
  let uniformsSet = false;
  function frame(now) {
    const keys = input.read();
    game.advance(now, keys.bits, keys.dx, keys.dy);
    const h = JSON.parse(game.hud());
    showMode(h);
    if (h.mode === "play" && h.play) {
      hud.update(h.play);
      el("engage").hidden = input.engaged();
    }
    for (const s of h.sounds) play(s);
    for (const t of h.toasts) hud.toast(t);
    if (renderer) {
      if (!uniformsSet) {
        renderer.setUniforms(game.uniforms(), game.clear_color());
        uniformsSet = true;
      }
      if (game.scenery_version() !== sceneryVersion) {
        sceneryVersion = game.scenery_version();
        renderer.setScenery(game.scenery());
      }
      renderer.frame(game.mats(), game.draws(), game.dynamic());
    }
    requestAnimationFrame(frame);
  }

  // tools and the browser check place the operative through this
  window.gatecrasher = { stage: (x, z, yaw) => game.stage(x, z, yaw) };
  document.body.dataset.ready = "true";
  requestAnimationFrame(frame);
}

start();
