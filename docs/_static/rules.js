// The pages come out of the build in their final shape, and this script only wires up their controls, so it never moves
// anything once a page shows. The template loads it in the head, so the rules page can call `setupRulesTable` right
// after its table, and the stylesheets see a page with scripts before the body paints: Furo drops this class only once
// the page has loaded, which would show its theme toggle and the example toggles late.
document.documentElement.classList.remove("no-js");

// The rules table streams in before the script after it can filter it, so with filters in the URL its rows stay out of
// the layout until they are applied: rows shown and then hidden would move everything around them.
const RULE_FILTERS = ["q", "group", "fix", "scope"];
if (RULE_FILTERS.some((key) => new URLSearchParams(window.location.search).get(key))) {
  document.documentElement.classList.add("rules-filtering");
}

// Filters from the URL hide rows before anything below the table exists; without any, the table stays as built.
function setupRulesTable(table) {
  const search = document.getElementById("rule-search");
  const group = document.getElementById("rule-group");
  const fix = document.getElementById("rule-fix");
  const scope = document.getElementById("rule-scope");
  const count = document.getElementById("rule-count");
  const empty = document.getElementById("rule-empty");
  const rows = [...table.querySelectorAll("tr.rule-row")];
  const groupRows = [...table.querySelectorAll("tr.group-row")];
  const params = new URLSearchParams(window.location.search);
  search.value = params.get("q") ?? "";
  group.value = params.get("group") ?? "";
  fix.value = params.get("fix") ?? "";
  scope.value = params.get("scope") ?? "";

  // the build sizes the box for the longest rule name; only text that no longer fits widens it, rather than scrolling
  // out of view
  const fit = () => {
    search.style.removeProperty("width");
    if (search.scrollWidth > search.clientWidth) {
      search.style.width = `${search.scrollWidth + search.offsetWidth - search.clientWidth}px`;
    }
  };
  fit();

  const apply = () => {
    fit();
    const words = search.value.toLowerCase().split(/\s+/).filter(Boolean);
    let shown = 0;
    for (const row of rows) {
      const visible =
        words.every((word) => row.dataset.text.includes(word)) &&
        (!group.value || row.dataset.group === group.value) &&
        (!fix.value || row.dataset.fix === fix.value) &&
        (!scope.value || row.dataset.scope === scope.value);
      row.hidden = !visible;
      shown += visible ? 1 : 0;
    }
    for (const header of groupRows) {
      header.hidden = !rows.some((row) => !row.hidden && row.dataset.group === header.dataset.group);
    }
    count.textContent = shown === rows.length ? `${rows.length} rules` : `${shown} of ${rows.length} rules`;
    empty.hidden = shown !== 0;
    const state = new URLSearchParams();
    const values = [search.value, group.value, fix.value, scope.value];
    for (const [key, value] of RULE_FILTERS.map((key, index) => [key, values[index]])) {
      if (value) {
        state.set(key, value);
      }
    }
    const query = state.toString();
    history.replaceState(null, "", query ? `?${query}` : window.location.pathname);
  };

  for (const control of [search, group, fix, scope]) {
    control.addEventListener("input", apply);
  }
  // typing a "/" into another field, or pressing a shortcut with a modifier, is not a request to search
  const typing = new Set(["INPUT", "TEXTAREA", "SELECT"]);
  document.addEventListener("keydown", (event) => {
    if (event.key !== "/" || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) {
      return;
    }
    if (typing.has(event.target.tagName) || event.target.isContentEditable) {
      return;
    }
    event.preventDefault();
    search.focus();
  });
  if (search.value || group.value || fix.value || scope.value) {
    apply();
  }
  document.documentElement.classList.remove("rules-filtering");
}

// each example's icon, placed by the build, toggles its dropdown
function linkExamples() {
  for (const button of document.querySelectorAll("button.example-toggle")) {
    const example = document.getElementById(button.getAttribute("aria-controls"));
    button.addEventListener("click", () => {
      example.open = !example.open;
    });
    example.addEventListener("toggle", () => {
      button.setAttribute("aria-expanded", String(example.open));
    });
  }
}

// one pill in the badge row opens every example, or closes them all once each is open
function linkExampleSwitch() {
  const button = document.querySelector("button.example-switch");
  if (!button) {
    return;
  }
  const examples = [...document.querySelectorAll("details.example:not(.example-static)")];
  const update = () => {
    const allOpen = examples.every((example) => example.open);
    button.textContent = allOpen ? "Collapse all examples" : "Expand all examples";
    button.setAttribute("aria-expanded", String(allOpen));
  };
  button.addEventListener("click", () => {
    const open = !examples.every((example) => example.open);
    for (const example of examples) {
      example.open = open;
    }
  });
  for (const example of examples) {
    example.addEventListener("toggle", update);
  }
}

document.addEventListener("DOMContentLoaded", () => {
  linkExamples();
  linkExampleSwitch();
});
