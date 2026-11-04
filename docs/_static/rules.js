// Furo links the current page as `#`, so the sidebar icons key off the page each entry names instead. Rule pages are
// skipped: they lead with their code and take no icon.
function markSidebarPages() {
  for (const link of document.querySelectorAll(".sidebar-tree a.reference")) {
    const path = link.getAttribute("href") === "#" ? location.pathname : new URL(link.href).pathname;
    if (path.includes("/_generated/rules/")) {
      continue;
    }
    // a folder's own index page is named after the folder, so it does not take the home page's icon
    const parts = path.replace(/\/$/, "/index.html").split("/");
    const page = parts.pop().replace(/\.html$/, "");
    link.dataset.page = page === "index" && parts.at(-1) ? parts.at(-1) : page || "index";
  }
}

function setupRulesTable(table) {
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
  apply();
}

// each example dropdown of a rule page gets an icon at the end of its list item's first paragraph, which toggles it
const EXAMPLE_TOGGLES = {
  "example-reported": ["✗", "Show an example it reports"],
  "example-not-reported": ["✓", "Show an example it accepts"],
  "example-pair": ["✗✓", "Show an example it reports beside one it accepts"],
  "example-fix": ["🔧", "Show the fix"],
  "example-configuration": ["⚙", "Show the configuration"],
};

function linkExamples(examples) {
  let count = 0;
  for (const example of examples) {
    const kind = Object.keys(EXAMPLE_TOGGLES).find((name) => example.classList.contains(name));
    const paragraph = example.closest("li")?.querySelector(":scope > p");
    if (!kind || !paragraph) {
      continue;
    }
    const [icon, label] = EXAMPLE_TOGGLES[kind];
    example.id ||= `rule-example-${++count}`;
    const button = document.createElement("button");
    button.type = "button";
    button.className = `example-toggle ${kind}`;
    // a pair's icon is both marks, colored apart by the stylesheet
    for (const mark of icon) {
      const span = document.createElement("span");
      if (kind === "example-pair") {
        span.className = `mark-${mark === "✗" ? "reported" : "not-reported"}`;
      }
      span.textContent = mark;
      button.append(span);
    }
    button.title = label;
    button.setAttribute("aria-label", label);
    button.setAttribute("aria-controls", example.id);
    button.setAttribute("aria-expanded", String(example.open));
    button.addEventListener("click", () => {
      example.open = !example.open;
    });
    example.addEventListener("toggle", () => {
      button.setAttribute("aria-expanded", String(example.open));
    });
    paragraph.append(button);
    example.classList.add("example-inline");
  }
}

// one pill in the badge row opens every example, or closes them all once each is open
function addExampleSwitch(examples) {
  const badges = document.querySelector(".badges");
  if (!badges) {
    return;
  }
  const button = document.createElement("button");
  button.type = "button";
  button.className = "badge example-switch";
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
  update();
  badges.append(button);
}

// The highlighter writes a numbered snippet as one run of text with the numbers inline, so a wrapped line would continue
// under the numbers. Splitting it into one block per line lets the stylesheet hang each continuation under its code.
function splitCodeLines(pre) {
  const lines = [[]];
  for (const node of [...pre.childNodes]) {
    const parts = node.textContent.split("\n");
    parts.forEach((part, index) => {
      if (index > 0) {
        lines.push([]);
      }
      if (part === "" && parts.length > 1) {
        return;
      }
      const piece = node.nodeType === Node.TEXT_NODE ? document.createTextNode(part) : node.cloneNode(false);
      if (piece.nodeType !== Node.TEXT_NODE) {
        piece.textContent = part;
      }
      lines.at(-1).push(piece);
    });
  }
  if (lines.at(-1).length === 0) {
    lines.pop();
  }
  pre.replaceChildren(
    ...lines.map((nodes) => {
      const line = document.createElement("span");
      line.className = "code-line";
      line.append(...nodes);
      return line;
    }),
  );
}

// a closed dropdown's snippets are split when it first opens, since nothing shows their lines before then
function splitExampleCode(example) {
  for (const pre of example.querySelectorAll("pre")) {
    if (pre.querySelector(".linenos") && !pre.querySelector(".code-line")) {
      splitCodeLines(pre);
    }
  }
}

// A fix's before and after, or a reported example and its accepted one, sit side by side only while every line fits
// its half; otherwise they stack, and only lines too long even then wrap. Examples are measured together, so the page
// reflows once: all open examples take the measuring class before any is read, and each is then toggled.
function layoutSides(examples) {
  const open = examples.filter((example) => example.open);
  for (const example of open) {
    example.classList.remove("example-stacked");
    example.classList.add("example-measuring");
  }
  const tooWide = open.map((example) =>
    [...example.querySelectorAll(".example-side pre")].some((pre) => pre.scrollWidth > pre.clientWidth + 1),
  );
  open.forEach((example, index) => {
    example.classList.remove("example-measuring");
    example.classList.toggle("example-stacked", tooWide[index]);
  });
}

function initExamples() {
  const examples = [...document.querySelectorAll("details.example")];
  if (examples.length > 0) {
    linkExamples(examples);
    addExampleSwitch(examples);
  }
  for (const example of examples) {
    example.addEventListener("toggle", () => {
      if (example.open) {
        splitExampleCode(example);
      }
    });
  }
  for (const block of document.querySelectorAll(".example:not(details) pre")) {
    if (block.querySelector(".linenos")) {
      splitCodeLines(block);
    }
  }
  const sided = [...document.querySelectorAll("details.example-fix, details.example-pair")];
  for (const example of sided) {
    example.addEventListener("toggle", () => layoutSides([example]));
  }
  layoutSides(sided);
  let pending = 0;
  addEventListener("resize", () => {
    cancelAnimationFrame(pending);
    pending = requestAnimationFrame(() => layoutSides(sided));
  });
}

document.addEventListener("DOMContentLoaded", () => {
  markSidebarPages();
  const table = document.querySelector("table.rules");
  if (table) {
    setupRulesTable(table);
  }
  initExamples();
});
