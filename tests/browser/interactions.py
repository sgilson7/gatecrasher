"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. These drive the game the way a person would and assert what the page
shows. Expected values are worked out from the data files, never copied from
the game's own output:

- the title lists missions.json's three missions by name;
- a mission's briefing shows its name and brief, and Deploy starts it with
  only the high-level objective known (its `objective_unknown` line);
- T switches the pistol between copy.json's tranquilliser and lethal names;
  C stands up from the starting crouch;
- standing still in the open 10 m in front of the first mission's first
  soldier (route [-3, 74], facing 180, i.e. looking south through the gate)
  gets you spotted, then shot: the failed screen gives copy.json's cause_shot;
- Esc pauses and Resume returns; the uniform chosen is kept after a reload.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
COPY = json.loads((ROOT / "data" / "copy.json").read_text())
MISSIONS = json.loads((ROOT / "data" / "missions.json").read_text())["missions"]


def click(page, selector):
    page.click(selector, timeout=5000)
    page.wait_for_timeout(250)


def mode(page):
    return page.evaluate("() => document.body.dataset.mode")


def engage(page):
    if not page.is_hidden("#engage"):
        click(page, "#engage-btn")


def run(page, check, engine):
    message = page.inner_text("#message")
    check(message in ("", COPY["no_webgl"]), f"no error message is shown ({message!r})")
    check(mode(page) == "title", f"the game opens on the title ({mode(page)})")
    check(page.text_content("#title h1") == COPY["title"], "the title from data/copy.json is drawn")
    names = page.eval_on_selector_all("#mission-list .name", "els => els.map(e => e.textContent)")
    check(names == [m["name"] for m in MISSIONS], f"the three missions are listed: {names}")

    click(page, "#mission-list button:nth-child(1)")
    check(mode(page) == "brief", f"choosing a mission opens its briefing ({mode(page)})")
    check(page.text_content("#b-name") == MISSIONS[0]["name"], "the briefing names the mission")
    check(page.text_content("#b-brief") == MISSIONS[0]["brief"], "and gives its brief")
    click(page, "#uniforms tr:nth-child(3)")
    check("on" in page.get_attribute("#uniforms tr:nth-child(3)", "class"), "a uniform can be chosen")

    click(page, "#deploy")
    check(mode(page) == "play", f"Deploy starts the mission ({mode(page)})")
    engage(page)
    page.wait_for_timeout(400)
    check(page.inner_text("#h-objective") == MISSIONS[0]["objective"], "the objective is shown")
    check(page.inner_text("#h-note") == MISSIONS[0]["objective_unknown"], "and where to find it is unknown")
    check(page.inner_text("#h-rounds") == COPY["rounds_tranq"], "the pistol starts with tranquilliser rounds")
    page.keyboard.press("KeyT")
    page.wait_for_timeout(200)
    check(page.inner_text("#h-rounds") == COPY["rounds_lethal"], "T switches to lethal rounds")
    page.keyboard.press("KeyT")
    page.wait_for_timeout(200)
    check(page.inner_text("#h-stance") == COPY["stance_crouch"], "she starts crouching")
    page.keyboard.press("KeyC")
    page.wait_for_timeout(200)
    check(page.inner_text("#h-stance") == COPY["stance_stand"], f"C stands her up: {page.inner_text('#h-stance')!r}")

    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    check(mode(page) == "paused", f"Esc pauses ({mode(page)})")
    click(page, "#resume")
    engage(page)
    check(mode(page) == "play", f"and Resume goes back ({mode(page)})")

    guard = MISSIONS[0]["soldiers"][0]["route"][0]
    page.evaluate(f"() => window.gatecrasher.stage({guard[0]}, {guard[1] + 10}, 0)")
    try:
        page.wait_for_function("() => document.body.dataset.mode === 'failed'", timeout=90000)
        shot = True
    except Exception:
        shot = False
    check(shot, f"standing in a soldier's sight gets you shot ({mode(page)})")
    check(page.inner_text("#f-cause") == COPY["cause_shot"], f"one bullet: {page.inner_text('#f-cause')!r}")
    click(page, "#retry")
    check(mode(page) == "play", f"Try again restarts the mission ({mode(page)})")
    page.keyboard.press("Escape")
    page.wait_for_timeout(300)
    click(page, "#abort")
    check(mode(page) == "title", f"Abort goes to the title ({mode(page)})")
    click(page, "#to-graceland")
    check(mode(page) == "graceland", f"Graceland opens ({mode(page)})")
    check(page.inner_text("#g-roster") == COPY["roster_empty"], "with nobody on staff yet")
    click(page, "#graceland-back")

    page.reload()
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    click(page, "#mission-list button:nth-child(2)")
    check("on" in (page.get_attribute("#uniforms tr:nth-child(3)", "class") or ""), "the uniform was kept after a reload")
