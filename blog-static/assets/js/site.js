const root = document.documentElement;

function applyTheme(theme) {
  const dark = theme === "dark" || (theme === "auto" && matchMedia("(prefers-color-scheme: dark)").matches);
  root.classList.toggle("dark", dark);
  root.dataset.theme = theme;
}

document.querySelectorAll("[data-theme-toggle]").forEach((button) => {
  button.addEventListener("click", () => {
    const theme = root.classList.contains("dark") ? "light" : "dark";
    localStorage.setItem("theme", theme);
    applyTheme(theme);
  });
});

matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
  if ((localStorage.getItem("theme") || "auto") === "auto") applyTheme("auto");
});

document.querySelectorAll("[data-copy-code]").forEach((button) => {
  button.classList.remove("hidden");
  button.classList.add("inline-flex");

  button.addEventListener("click", async () => {
    const code = button.closest(".code-frame")?.querySelector("code")?.innerText;
    if (!code) return;

    const label = button.querySelector("[data-copy-label]");
    try {
      await navigator.clipboard.writeText(code);
      label.textContent = "Copied";
    } catch {
      label.textContent = "Copy failed";
    }
    setTimeout(() => label.textContent = "Copy", 1600);
  });
});

document.querySelectorAll("[data-copy-fediverse]").forEach((button) => {
  button.addEventListener("click", async () => {
    const label = button.querySelector("[data-fediverse-copy-label]");
    try {
      await navigator.clipboard.writeText(button.dataset.copyFediverse);
      label.textContent = "Copied";
    } catch {
      label.textContent = "Copy failed";
    }
    setTimeout(() => label.textContent = "Copy handle", 1600);
  });
});

document.querySelectorAll("[data-discussion-section]").forEach(async (section) => {
  const list = section.querySelector("ul");
  const knownUrls = new Set(
    [...list.querySelectorAll("[data-discussion-url]")].map((item) => item.dataset.discussionUrl),
  );

  function addLink(link) {
    if (!link?.url || knownUrls.has(link.url)) return;

    let url;
    try {
      url = new URL(link.url);
    } catch {
      return;
    }
    if (url.protocol !== "https:" && url.protocol !== "http:") return;

    const item = document.createElement("li");
    const anchor = document.createElement("a");
    const label = document.createElement("span");
    const accessibleLabel = document.createElement("span");
    const icon = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");

    item.dataset.discussionUrl = link.url;
    anchor.href = link.url;
    anchor.rel = "noopener noreferrer";
    anchor.className = "inline-flex min-h-11 items-center gap-2 rounded-xl border border-line bg-panel-raised px-4 py-2 text-sm font-bold text-ink transition-colors hover:border-accent hover:bg-accent-soft hover:text-accent";
    label.textContent = link.label || link.source || "Discussion";
    icon.setAttribute("viewBox", "0 0 24 24");
    icon.setAttribute("class", "size-4");
    icon.setAttribute("fill", "none");
    icon.setAttribute("stroke", "currentColor");
    icon.setAttribute("stroke-width", "1.8");
    icon.setAttribute("aria-hidden", "true");
    path.setAttribute("d", "M14 5h5v5M19 5l-8 8M19 13v6H5V5h6");
    accessibleLabel.className = "sr-only";
    accessibleLabel.textContent = "on another site";
    icon.append(path);
    anchor.append(label, icon, accessibleLabel);
    item.append(anchor);
    list.append(item);
    knownUrls.add(link.url);
  }

  section.setAttribute("aria-busy", "true");
  try {
    const apiBase = section.dataset.discussionApiBase.replace(/\/$/, "");
    const slug = encodeURIComponent(section.dataset.discussionSlug);
    const response = await fetch(`${apiBase}/discussions/${slug}`);
    if (response.ok) {
      const data = await response.json();
      if (Array.isArray(data.links)) data.links.forEach(addLink);
      const replies = Number.isInteger(data.replies) ? data.replies : 0;
      const boosts = Number.isInteger(data.boosts) ? data.boosts : 0;
      const activity = section.querySelector("[data-fediverse-activity]");
      if (activity && (replies > 0 || boosts > 0)) {
        activity.querySelector("[data-reply-count]").textContent = `${replies} ${replies === 1 ? "reply" : "replies"}`;
        activity.querySelector("[data-boost-count]").textContent = `${boosts} ${boosts === 1 ? "boost" : "boosts"}`;
        activity.classList.remove("hidden");
      }
    }
  } catch {
    // Front matter links remain usable when the optional backend is unavailable.
  } finally {
    section.classList.toggle("hidden", knownUrls.size === 0);
    section.removeAttribute("aria-busy");
  }
});
