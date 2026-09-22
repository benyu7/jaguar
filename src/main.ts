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
  verificationUri = null;
  showAuthError(null);
  showAuth("signed-out");
}

async function restoreSession() {
  // The token lives in the Rust process, so it survives a webview reload even
  // though it does not survive a restart.
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
  restoreSession();
});
