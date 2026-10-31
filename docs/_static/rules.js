document.addEventListener("DOMContentLoaded", () => {
  const table = document.querySelector("table.rules");
  if (!table) {
    return;
  }
  const search = document.getElementById("rule-search");
  const group = document.getElementById("rule-group");
  const fix = document.getElementById("rule-fix");
  const count = document.getElementById("rule-count");
  const empty = document.getElementById("rule-empty");
  const rows = [...table.querySelectorAll("tr.rule-row")];
  const groupRows = [...table.querySelectorAll("tr.group-row")];
  const params = new URLSearchParams(window.location.search);
  search.value = params.get("q") ?? "";
  group.value = params.get("group") ?? "";
  fix.value = params.get("fix") ?? "";

  const apply = () => {
    const words = search.value.toLowerCase().split(/\s+/).filter(Boolean);
    let shown = 0;
    for (const row of rows) {
      const visible =
        words.every((word) => row.dataset.text.includes(word)) &&
        (!group.value || row.dataset.group === group.value) &&
        (!fix.value || row.dataset.fix === fix.value);
      row.hidden = !visible;
      shown += visible ? 1 : 0;
    }
    for (const header of groupRows) {
      header.hidden = !rows.some((row) => !row.hidden && row.dataset.group === header.dataset.group);
    }
    count.textContent = shown === rows.length ? `${rows.length} rules` : `${shown} of ${rows.length} rules`;
    empty.hidden = shown !== 0;
    const state = new URLSearchParams();
    for (const [key, value] of [["q", search.value], ["group", group.value], ["fix", fix.value]]) {
      if (value) {
        state.set(key, value);
      }
    }
    const query = state.toString();
    history.replaceState(null, "", query ? `?${query}` : window.location.pathname);
  };

  for (const control of [search, group, fix]) {
    control.addEventListener("input", apply);
  }
  document.addEventListener("keydown", (event) => {
    if (event.key === "/" && document.activeElement !== search) {
      event.preventDefault();
      search.focus();
    }
  });
  apply();
});
