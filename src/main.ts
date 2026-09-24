import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

type DevicePrompt = {
  user_code: string;
  verification_uri: string;
  expires_in: number;
};

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
  updated_at: string;
  draft: boolean;
};

type AuthView = "signed-out" | "pending" | "signed-in";

// The URL GitHub told us to send the user to, kept so "Open GitHub again"
// works if they closed the tab.
let verificationUri: string | null = null;

function panel(view: AuthView): HTMLElement | null {
  return document.querySelector(`#auth-${view}`);
}

function showAuth(view: AuthView) {
  for (const name of ["signed-out", "pending", "signed-in"] as AuthView[]) {
    const el = panel(name);
    if (el) el.hidden = name !== view;
  }

  const prs = document.querySelector<HTMLElement>("#prs");
  if (prs && view !== "signed-in") prs.hidden = true;
}

function showAuthError(message: string | null) {
  const el = document.querySelector<HTMLElement>("#auth-error");
  if (!el) return;
  el.textContent = message ?? "";
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

/** "3 hours ago" for a GitHub timestamp, in the largest unit that fits. */
function relativeTime(timestamp: string): string {
  const seconds = (Date.parse(timestamp) - Date.now()) / 1000;
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ["year", 60 * 60 * 24 * 365],
    ["month", 60 * 60 * 24 * 30],
    ["day", 60 * 60 * 24],
    ["hour", 60 * 60],
    ["minute", 60],
  ];
  const format = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

  for (const [unit, size] of units) {
    if (Math.abs(seconds) >= size) {
      return format.format(Math.round(seconds / size), unit);
    }
  }
  return format.format(Math.round(seconds), "second");
}

function pullRequestItem(pr: PullRequest): HTMLLIElement {
  const item = document.createElement("li");
  item.className = "pr";

  // A plain href would navigate the webview itself, so the click handler sends
  // it to the real browser instead; the href is still there so the link reads
  // as one and shows its target on hover.
  const link = document.createElement("a");
  link.className = "pr-title";
  link.href = pr.url;
  link.textContent = pr.title;
  link.addEventListener("click", (event) => {
    event.preventDefault();
    openUrl(pr.url).catch(() => showPrStatus("Could not open a browser."));
  });
  item.append(link);

  if (pr.draft) {
    const draft = document.createElement("span");
    draft.className = "pr-draft";
    draft.textContent = "Draft";
    item.append(draft);
  }

  const meta = document.createElement("span");
  meta.className = "pr-meta";
  meta.textContent = `${pr.repository} #${pr.number} · updated ${relativeTime(pr.updated_at)}`;
  item.append(meta);

  return item;
}

async function loadPullRequests() {
  const section = document.querySelector<HTMLElement>("#prs");
  const list = document.querySelector<HTMLUListElement>("#pr-list");
  const refresh = document.querySelector<HTMLButtonElement>("#refresh-prs");
  if (!section || !list) return;

  section.hidden = false;
  if (refresh) refresh.disabled = true;
  showPrStatus("Loading…");

  try {
    const prs = await invoke<PullRequest[]>("list_my_pull_requests");
    list.replaceChildren(...prs.map(pullRequestItem));
    showPrStatus(prs.length === 0 ? "No open pull requests." : null);
  } catch (error) {
    list.replaceChildren();
    showPrStatus(String(error));
  } finally {
    if (refresh) refresh.disabled = false;
  }
}

async function openVerificationUri() {
  if (!verificationUri) return;
  try {
    await openUrl(verificationUri);
  } catch {
    // A browser that refuses to launch is not fatal — the user can still
    // reach github.com/login/device by hand while we keep polling.
    showAuthError(`Could not open a browser. Go to ${verificationUri} yourself.`);
  }
}

async function signIn() {
  const button = document.querySelector<HTMLButtonElement>("#sign-in");
  if (button) button.disabled = true;
  showAuthError(null);

  try {
    const prompt = await invoke<DevicePrompt>("start_device_auth");
    verificationUri = prompt.verification_uri;

    const code = document.querySelector("#user-code");
    if (code) code.textContent = prompt.user_code;
    showAuth("pending");

    await openVerificationUri();

    // Resolves only once GitHub gives a final answer, so this await is the
    // whole polling loop.
    showUser(await invoke<User>("complete_device_auth"));
  } catch (error) {
    showAuthError(String(error));
    showAuth("signed-out");
  } finally {
    if (button) button.disabled = false;
  }
}

async function signOut() {
  await invoke("sign_out");
  document.querySelector("#pr-list")?.replaceChildren();
  showPrStatus(null);
  showAuthError(null);
  showAuth("signed-out");
}

async function restoreSession() {
  // The token lives in the Rust process, backed by the OS credential store, so
  // this recovers the session across both a webview reload and a restart. It
  // returns null if the token has since been revoked.
  try {
    const user = await invoke<User | null>("current_user");
    if (user) showUser(user);
    else showAuth("signed-out");
  } catch {
    showAuth("signed-out");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  document.querySelector("#sign-in")?.addEventListener("click", signIn);
  document.querySelector("#sign-out")?.addEventListener("click", signOut);
  document
    .querySelector("#open-github")
    ?.addEventListener("click", openVerificationUri);
  document
    .querySelector("#refresh-prs")
    ?.addEventListener("click", loadPullRequests);
  restoreSession();
});
