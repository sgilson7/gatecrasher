// The play HUD: draws what the core's HUD JSON says. Words come from
// data/copy.json (keys) or from the core (mission text); numbers come from
// the core. Nothing here decides anything about the game.

const el = (id) => document.getElementById(id);

// "@key" or "@key:argument" is a line from copy.json; anything else is shown as it is.
export function words(copy, text) {
  if (!text.startsWith("@")) return text;
  const [key, ...rest] = text.slice(1).split(":");
  const line = copy[key] || key;
  return rest.length ? line.replace("{}", rest.join(":")) : line;
}

function pool(parent, make) {
  const nodes = [];
  return (n) => {
    while (nodes.length < n) {
      const node = make();
      parent.appendChild(node);
      nodes.push(node);
    }
    nodes.forEach((node, k) => {
      node.hidden = k >= n;
    });
    return nodes;
  };
}

export function makeHud(copy, display) {
  const markers = pool(el("markers"), () => {
    const d = document.createElement("div");
    d.className = "marker";
    d.append(document.createElement("i"), document.createElement("span"));
    return d;
  });
  const ticks = pool(el("detect"), () => {
    const d = document.createElement("div");
    d.className = "tick";
    d.append(document.createElement("div"));
    return d;
  });
  const toastBox = el("toasts");
  let intelShown = "";

  function toast(text) {
    const d = document.createElement("div");
    d.textContent = words(copy, text);
    toastBox.prepend(d);
    while (toastBox.children.length > display.toast_max) toastBox.lastChild.remove();
    setTimeout(() => d.remove(), display.toast_ms);
  }

  return {
    toast,
    toggleIntel() {
      el("intel").hidden = !el("intel").hidden;
    },
    update(p) {
      el("h-mission").textContent = p.mission;
      el("h-time").textContent = p.time;
      const phase = el("phase");
      phase.hidden = p.phase === "normal";
      phase.className = p.phase;
      el("h-phase").textContent = copy[`phase_${p.phase}`];
      el("h-phase-secs").textContent = p.phase === "caution" || p.phase === "evasion" ? p.phase_secs : "";
      el("hud").classList.toggle("combat", p.phase === "combat");
      el("reflex").hidden = !p.reflex;
      el("h-camo").textContent = `${p.camo}%`;
      el("h-uniform").textContent = p.changing ? copy.changing : p.uniform;
      el("h-stance").textContent = copy[`stance_${p.stance}`];
      el("h-battery").style.width = `${p.battery}%`;
      el("h-battery").style.opacity = p.cloak ? "1" : "0.55";
      el("h-fultons").textContent = p.fultons;
      el("h-mule").textContent = p.mule_loaded ? copy.mule_loaded : p.mule_grace ? copy.mule_grace : "";
      const rounds = el("h-rounds");
      rounds.textContent = copy[`rounds_${p.rounds}`];
      rounds.parentElement.classList.toggle("lethal", p.rounds === "lethal");
      el("h-ammo").textContent = p.ammo;
      const [blue, orange] = p.portals;
      el("h-pa").classList.toggle("on", blue);
      el("h-pb").classList.toggle("on", orange);
      el("reticle").classList.toggle("aim", p.aim);
      const obj = el("h-objective");
      obj.textContent = p.objective;
      obj.classList.toggle("done", p.objective_done);
      el("h-note").textContent = p.objective_note;
      // prompts
      const box = el("prompts");
      const want = p.prompts.filter((k) => k).map((k) => copy[k] || k);
      if (box.dataset.shown !== want.join("|")) {
        box.dataset.shown = want.join("|");
        box.replaceChildren(
          ...want.map((t) => {
            const d = document.createElement("div");
            d.textContent = t;
            const wrap = document.createElement("div");
            wrap.style.background = "none";
            wrap.style.border = "none";
            wrap.append(d);
            return wrap;
          }),
        );
      }
      const said = p.said_key ? copy[p.said_key] : p.said_text;
      el("said").hidden = !said;
      el("said").textContent = said || "";
      // intel
      const intelNow = p.intel.join("\n");
      if (intelNow !== intelShown) {
        intelShown = intelNow;
        const list = el("intel-list");
        const items = p.intel.length ? p.intel : [copy.intel_none];
        list.replaceChildren(
          ...items.map((t) => {
            const li = document.createElement("li");
            li.textContent = t;
            return li;
          }),
        );
      }
      // markers over the world
      const ms = markers(p.markers.length);
      p.markers.forEach(([x, y, kind, dist], k) => {
        const node = ms[k];
        node.className = `marker ${kind}`;
        node.style.left = `${x}%`;
        node.style.top = `${y}%`;
        node.lastChild.textContent = dist ? `${dist}${copy.unit_m}` : "";
      });
      // the detection ticks round the middle of the screen
      const ts = ticks(p.detect.length);
      p.detect.forEach(([angle, level], k) => {
        const node = ts[k];
        node.className = `tick ${level}`;
        node.style.transform = `rotate(${angle}deg)`;
      });
    },
  };
}
