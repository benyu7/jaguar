import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

type User = {
  login: string;
  name: string | null;
  avatar_url: string;
};

type PullRequest = {
  number: number;
  title: string;
  url: string;
  repository: string;
  /** Already worded by the backend — "3 hours ago". */
  updated: string;
  draft: boolean;
};

const AUTH_VIEWS = ["checking", "signed-in", "unavailable"] as const;
type AuthView = (typeof AUTH_VIEWS)[number];

function showAuth(view: AuthView) {
  for (const name of AUTH_VIEWS) {
    const el = document.querySelector<HTMLElement>(`#auth-${name}`);
    if (el) el.hidden = name !== view;
  }

  const prs = document.querySelector<HTMLElement>("#prs");
  if (prs && view !== "signed-in") prs.hidden = true;
}

function showAuthError(message: string | null) {
  const el = document.querySelector<HTMLElement>("#auth-error");
  if (!el) return;

  // The messages carry a "Run: gh auth login" line of their own, and nothing
  // here preserves newlines, so the breaks have to be real elements.
  const lines = (message ?? "").split("\n");
  el.replaceChildren(
    ...lines.flatMap((line, i) =>
      i === 0 ? [line] : [document.createElement("br"), line],
    ),
  );
  el.hidden = message === null;
}

function showUser(user: User) {
  const avatar = document.querySelector<HTMLImageElement>("#avatar");
  const who = document.querySelector("#who");
  if (avatar) {
    avatar.src = user.avatar_url;
    avatar.alt = `${user.login}'s avatar`;
  }
  if (who) who.textContent = user.name ?? user.login;
  showAuth("signed-in");
  loadPullRequests();
}

function showPrStatus(message: string | null) {
  const el = document.querySelector<HTMLElement>("#prs-status");
  if (!el) return;
  el.textContent = message ?? "";
  el.hidden = message === null;
}

function pullRequestRow(pr: PullRequest): HTMLTableRowElement {
  const row = document.createElement("tr");
  const title = document.createElement("td");

  // A plain href would navigate the webview itself, so the click handler sends
  // it to the real browser instead; the href is still there so the link reads
  // as one and shows its target on hover.
  const link = document.createElement("a");
  link.href = pr.url;
  link.textContent = pr.title;
  link.addEventListener("click", (event) => {
    event.preventDefault();
    openUrl(pr.url).catch(() => showPrStatus("Could not open a browser."));
  });
  title.append(link);

  if (pr.draft) {
    // Pico gives <mark> a padded, highlighted face — a badge without a class.
    const draft = document.createElement("mark");
    draft.textContent = "Draft";
    title.append(" ", draft);
  }

  const meta = document.createElement("small");
  meta.textContent = `${pr.repository} #${pr.number}`;
  title.append(document.createElement("br"), meta);
  row.append(title);

  const updated = document.createElement("td");
  const age = document.createElement("small");
  age.textContent = pr.updated;
  updated.append(age);
  row.append(updated);

  return row;
}

async function loadPullRequests() {
  const section = document.querySelector<HTMLElement>("#prs");
  const table = document.querySelector<HTMLTableElement>("#pr-table");
  const list = document.querySelector<HTMLTableSectionElement>("#pr-list");
  const refresh = document.querySelector<HTMLButtonElement>("#refresh-prs");
  if (!section || !list) return;

  section.hidden = false;
  if (refresh) refresh.disabled = true;
  showPrStatus("Loading…");

  try {
    const prs = await invoke<PullRequest[]>("list_my_pull_requests");
    list.replaceChildren(...prs.map(pullRequestRow));
    // A lone header row over nothing reads as a fault rather than an empty list.
    if (table) table.hidden = prs.length === 0;
    showPrStatus(prs.length === 0 ? "No open pull requests." : null);
  } catch (error) {
    list.replaceChildren();
    if (table) table.hidden = true;
    showPrStatus(String(error));
  } finally {
    if (refresh) refresh.disabled = false;
  }
}

/// There is no sign-in of our own to do — `gh` is either signed in or it is
/// not, and the error says which and how to fix it. "Try again" re-asks, so a
/// `gh auth login` in a terminal is picked up without restarting the app.
async function identify() {
  const button = document.querySelector<HTMLButtonElement>("#retry");
  if (button) button.disabled = true;
  showAuth("checking");

  try {
    showUser(await invoke<User>("current_user"));
  } catch (error) {
    showAuthError(String(error));
    showAuth("unavailable");
  } finally {
    if (button) button.disabled = false;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  document.querySelector("#retry")?.addEventListener("click", identify);
  document
    .querySelector("#refresh-prs")
    ?.addEventListener("click", loadPullRequests);
  identify();
});
