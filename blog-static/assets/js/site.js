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
  const list = section.querySelector("[data-discussion-links]");
  const repliesSection = section.querySelector("[data-fediverse-replies]");
  const repliesList = section.querySelector("[data-fediverse-reply-list]");
  let renderedReplies = 0;
  const knownUrls = new Set(
    [...list.querySelectorAll("[data-discussion-url]")].map((item) => item.dataset.discussionUrl),
  );
  const knownSources = new Set(
    [...list.querySelectorAll("[data-discussion-source]")].map((item) => item.dataset.discussionSource),
  );

  function addLink(link) {
    const source = link?.source || "discussion";
    if (!link?.url || knownUrls.has(link.url) || knownSources.has(source)) return;

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

    item.dataset.discussionSource = source;
    item.dataset.discussionUrl = link.url;
    anchor.href = link.url;
    anchor.rel = "noopener noreferrer";
    anchor.className = "inline-flex min-h-11 items-center gap-2 rounded-lg border border-line px-4 py-2 text-sm font-semibold text-ink shadow-sm transition-colors";
    if (source === "reddit") {
      anchor.classList.add("hover:border-[#ff4500]", "hover:bg-[#ff4500]/10");
    } else if (source === "hacker_news") {
      anchor.classList.add("hover:border-[#ff6600]", "hover:bg-[#ff6600]/10");
    } else if (source === "mastodon" || source === "fediverse") {
      anchor.classList.add("hover:border-[#6364ff]", "hover:bg-[#6364ff]/10");
    } else {
      anchor.classList.add("hover:border-accent", "hover:bg-accent-soft");
    }
    label.textContent = link.label || link.source || "Discussion";
    icon.setAttribute("viewBox", "0 0 24 24");
    icon.setAttribute("class", "size-5 shrink-0");
    icon.setAttribute("aria-hidden", "true");
    if (source === "reddit") {
      icon.classList.add("fill-[#ff4500]");
      icon.innerHTML = '<path d="M12 0C5.373 0 0 5.373 0 12c0 3.314 1.343 6.314 3.515 8.485l-2.286 2.286C.775 23.225 1.097 24 1.738 24H12c6.627 0 12-5.373 12-12S18.627 0 12 0Zm4.388 3.199c1.104 0 1.999.895 1.999 1.999 0 1.105-.895 2-1.999 2-.946 0-1.739-.657-1.947-1.539-1.147.162-2.032 1.15-2.032 2.341 1.776.067 3.4.567 4.686 1.363.473-.363 1.064-.58 1.707-.58 1.547 0 2.802 1.254 2.802 2.802 0 1.117-.655 2.081-1.601 2.531-.088 3.256-3.637 5.876-7.997 5.876-4.361 0-7.905-2.617-7.998-5.87-.954-.447-1.614-1.415-1.614-2.538 0-1.548 1.255-2.802 2.803-2.802.645 0 1.239.218 1.712.585 1.275-.79 2.881-1.291 4.64-1.365 0-1.663 1.263-3.034 2.88-3.207.188-.911.993-1.595 1.959-1.595ZM8.303 11.575c-.784 0-1.459.78-1.506 1.797-.047 1.016.64 1.429 1.426 1.429.786 0 1.371-.369 1.418-1.385.047-1.017-.553-1.841-1.338-1.841Zm7.406 0c-.786 0-1.385.824-1.338 1.841.047 1.017.634 1.385 1.418 1.385.785 0 1.473-.413 1.426-1.429-.046-1.017-.721-1.797-1.506-1.797Zm-3.703 4.013c-.974 0-1.907.048-2.77.135-.147.015-.241.168-.183.305.483 1.154 1.622 1.964 2.953 1.964 1.33 0 2.47-.81 2.953-1.964.057-.137-.037-.29-.184-.305-.863-.087-1.795-.135-2.769-.135Z"/>';
    } else if (source === "hacker_news") {
      icon.setAttribute("viewBox", "4 4 188 188");
      icon.innerHTML = '<path d="M4 4h188v188H4z" fill="#f60"/><path d="m73.252 45.01 22.748 47.391 22.748-47.391h19.566l-34.324 64.487v41.493H88.01v-41.493L53.686 45.01Z" fill="#fff"/>';
    } else if (source === "mastodon" || source === "fediverse") {
      icon.classList.add("fill-[#6364ff]");
      icon.innerHTML = '<path d="M23.268 5.313c-.35-2.578-2.617-4.61-5.304-5.004C17.51.242 15.792 0 11.813 0h-.03c-3.98 0-4.835.242-5.288.309C3.882.692 1.496 2.518.917 5.127.64 6.412.61 7.837.661 9.143c.074 1.874.088 3.745.26 5.611.118 1.24.325 2.47.62 3.68.55 2.237 2.777 4.098 4.96 4.857 2.336.792 4.849.923 7.256.38.265-.061.527-.132.786-.213.585-.184 1.27-.39 1.774-.753v-1.852a20.282 20.282 0 0 1-4.752.494c-2.73 0-3.463-1.284-3.674-1.818a5.593 5.593 0 0 1-.319-1.433c1.538.336 3.098.519 4.698.516.376 0 .75 0 1.125-.01 1.57-.044 3.224-.124 4.768-.422 2.473-.472 4.791-1.928 5.027-5.612.008-.145.03-1.52.03-1.67.002-.512.167-3.63-.024-5.545ZM19.52 14.508h-2.561V8.29c0-1.309-.55-1.976-1.67-1.976-1.23 0-1.846.79-1.846 2.35v3.403h-2.546V8.663c0-1.56-.617-2.35-1.848-2.35-1.112 0-1.668.668-1.67 1.977v6.218H4.822V8.102c0-1.31.337-2.35 1.011-3.12.696-.77 1.608-1.164 2.74-1.164 1.311 0 2.302.5 2.962 1.498l.638 1.06.638-1.06c.66-.999 1.65-1.498 2.96-1.498 1.13 0 2.043.395 2.74 1.164.675.77 1.012 1.81 1.012 3.12Z"/>';
    } else {
      icon.setAttribute("fill", "none");
      icon.setAttribute("stroke", "currentColor");
      icon.setAttribute("stroke-width", "1.8");
      icon.innerHTML = '<path d="M14 5h5v5M19 5l-8 8M19 13v6H5V5h6"/>';
    }
    accessibleLabel.className = "sr-only";
    accessibleLabel.textContent = "on another site";
    anchor.append(icon, label, accessibleLabel);
    item.append(anchor);
    list.append(item);
    knownUrls.add(link.url);
    knownSources.add(source);
  }

  function safeUrl(value) {
    try {
      const url = new URL(value);
      return url.protocol === "https:" || url.protocol === "http:" ? url.href : null;
    } catch {
      return null;
    }
  }

  function renderReplies(replies) {
    if (!repliesSection || !repliesList || !Array.isArray(replies)) return;

    const nodes = new Map();
    for (const reply of replies) {
      const id = safeUrl(reply?.id);
      const authorUrl = safeUrl(reply?.author_url);
      const replyUrl = safeUrl(reply?.url);
      if (!id || !authorUrl || !replyUrl || typeof reply.content !== "string") continue;

      const item = document.createElement("li");
      const article = document.createElement("article");
      const avatarLink = document.createElement("a");
      const avatar = document.createElement(reply.avatar_url ? "img" : "span");
      const body = document.createElement("div");
      const header = document.createElement("div");
      const identity = document.createElement("div");
      const author = document.createElement("a");
      const handle = document.createElement("span");
      const timestampLink = document.createElement("a");
      const timestamp = document.createElement("time");
      const content = document.createElement("div");

      item.dataset.replyId = id;
      article.className = "grid grid-cols-[2.75rem_minmax(0,1fr)] gap-3 py-5";
      avatarLink.href = authorUrl;
      avatarLink.rel = "nofollow noopener noreferrer";
      avatarLink.className = "row-span-2 block size-11 overflow-hidden rounded-xl bg-accent-soft";
      avatar.className = "flex size-11 items-center justify-center object-cover font-mono text-lg font-bold uppercase text-accent";
      if (reply.avatar_url) {
        const avatarUrl = safeUrl(reply.avatar_url);
        if (avatarUrl) {
          avatar.src = avatarUrl;
          avatar.alt = "";
          avatar.loading = "lazy";
          avatar.referrerPolicy = "no-referrer";
        } else {
          avatar.textContent = (reply.author_name || reply.author || "?").trim().charAt(0);
        }
      } else {
        avatar.textContent = (reply.author_name || reply.author || "?").trim().charAt(0);
      }
      body.className = "min-w-0";
      header.className = "flex min-w-0 items-start justify-between gap-3";
      identity.className = "min-w-0 leading-tight";
      author.href = authorUrl;
      author.rel = "nofollow noopener noreferrer";
      author.className = "block truncate text-sm font-bold text-ink hover:underline";
      author.textContent = reply.author_name || "Fediverse user";
      handle.className = "block truncate font-mono text-xs text-muted";
      handle.textContent = reply.author || "Fediverse user";
      timestampLink.href = replyUrl;
      timestampLink.rel = "nofollow noopener noreferrer";
      timestampLink.className = "shrink-0 text-xs text-muted hover:text-accent hover:underline";
      timestamp.className = "text-xs text-muted";
      if (reply.published_at) {
        const date = new Date(reply.published_at);
        if (!Number.isNaN(date.valueOf())) {
          timestamp.dateTime = date.toISOString();
          timestamp.title = new Intl.DateTimeFormat(undefined, { dateStyle: "long", timeStyle: "short" }).format(date);
          timestamp.textContent = new Intl.DateTimeFormat(undefined, { month: "short", day: "numeric" }).format(date);
        }
      }
      content.className = "prose prose-sm mt-3 max-w-none break-words text-ink dark:prose-invert";
      content.innerHTML = reply.content;
      content.querySelectorAll("a").forEach((link) => {
        link.target = "_blank";
        link.rel = "nofollow ugc noopener noreferrer";
      });
      body.append(header, content);
      if (reply.updated_at) {
        const edited = document.createElement("span");
        edited.className = "mt-2 block text-xs text-muted";
        edited.textContent = "Edited";
        body.append(edited);
      }

      avatarLink.append(avatar);
      identity.append(author, handle);
      timestampLink.append(timestamp);
      header.append(identity, timestampLink);
      article.append(avatarLink, body);
      item.append(article);
      nodes.set(id, { item, parent: safeUrl(reply.in_reply_to) });
    }

    for (const { item, parent } of nodes.values()) {
      const parentNode = parent ? nodes.get(parent) : null;
      if (!parentNode) {
        repliesList.append(item);
        continue;
      }
      let children = parentNode.item.querySelector(":scope > ol");
      if (!children) {
        children = document.createElement("ol");
        children.className = "ml-[1.375rem] border-l-2 border-line pl-8";
        children.setAttribute("role", "list");
        parentNode.item.append(children);
      }
      children.append(item);
    }
    renderedReplies = nodes.size;
    repliesSection.classList.toggle("hidden", renderedReplies === 0);
  }

  section.setAttribute("aria-busy", "true");
  try {
    const apiBase = section.dataset.discussionApiBase.replace(/\/$/, "");
    const slug = encodeURIComponent(section.dataset.discussionSlug);
    const response = await fetch(`${apiBase}/discussions/${slug}`);
    if (response.ok) {
      const data = await response.json();
      if (Array.isArray(data.links)) data.links.forEach(addLink);
      renderReplies(data.reply_items);
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
    section.classList.toggle("hidden", knownUrls.size === 0 && renderedReplies === 0);
    section.removeAttribute("aria-busy");
  }
});
