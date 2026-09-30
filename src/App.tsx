import { Fragment, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

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

type AuthState =
  | { status: "checking" }
  | { status: "signed-in"; user: User }
  | { status: "unavailable"; message: string };

function initials(user: User) {
  const source = (user.name ?? user.login).trim();
  const parts = source.split(/\s+/).filter(Boolean);
  if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase();
  return source.slice(0, 2).toUpperCase();
}

function AuthError({ message }: { message: string }) {
  // The messages carry a "Run: gh auth login" line of their own, and nothing
  // here preserves newlines, so the breaks have to be real elements.
  const lines = message.split("\n");
  return (
    <p className="text-sm">
      {lines.map((line, index) => (
        <Fragment key={index}>
          {index > 0 && <br />}
          {line}
        </Fragment>
      ))}
    </p>
  );
}

function PullRequestRow({
  pr,
  onOpenFailed,
}: {
  pr: PullRequest;
  onOpenFailed: () => void;
}) {
  return (
    <TableRow>
      <TableCell className="whitespace-normal">
        <div className="flex flex-wrap items-center gap-2">
          {/* A plain href would navigate the webview itself, so the click handler
              sends it to the real browser instead; the href is still there so the
              link reads as one and shows its target on hover. */}
          <a
            href={pr.url}
            className="underline-offset-4 hover:underline"
            onClick={(event) => {
              event.preventDefault();
              openUrl(pr.url).catch(onOpenFailed);
            }}
          >
            {pr.title}
          </a>
          {pr.draft && <Badge variant="secondary">Draft</Badge>}
        </div>
        <p className="text-xs text-muted-foreground">
          {pr.repository} #{pr.number}
        </p>
      </TableCell>
      <TableCell className="text-muted-foreground">{pr.updated}</TableCell>
    </TableRow>
  );
}

export function App() {
  const [auth, setAuth] = useState<AuthState>({ status: "checking" });
  const [pullRequests, setPullRequests] = useState<PullRequest[]>([]);
  const [prStatus, setPrStatus] = useState<string | null>(null);
  const [prLoading, setPrLoading] = useState(false);

  async function loadPullRequests() {
    setPrLoading(true);
    setPrStatus("Loading…");

    try {
      const prs = await invoke<PullRequest[]>("list_my_pull_requests");
      setPullRequests(prs);
      // A lone header row over nothing reads as a fault rather than an empty list.
      setPrStatus(prs.length === 0 ? "No open pull requests." : null);
    } catch (error) {
      setPullRequests([]);
      setPrStatus(String(error));
    } finally {
      setPrLoading(false);
    }
  }

  // There is no sign-in of our own to do — `gh` is either signed in or it is
  // not, and the error says which and how to fix it. "Try again" re-asks, so a
  // `gh auth login` in a terminal is picked up without restarting the app.
  async function identify() {
    setAuth({ status: "checking" });

    try {
      const user = await invoke<User>("current_user");
      setAuth({ status: "signed-in", user });
      await loadPullRequests();
    } catch (error) {
      setAuth({ status: "unavailable", message: String(error) });
    }
  }

  useEffect(() => {
    void identify();
  }, []);

  return (
    <div className="flex flex-col gap-8 p-6">
      <section>
        {auth.status === "checking" && <p>Asking the GitHub CLI who you are…</p>}
        {auth.status === "signed-in" && (
          <div className="flex items-center justify-center gap-3">
            <Avatar>
              <AvatarImage
                src={auth.user.avatar_url}
                alt={`${auth.user.login}'s avatar`}
              />
              <AvatarFallback>{initials(auth.user)}</AvatarFallback>
            </Avatar>
            <span>{auth.user.name ?? auth.user.login}</span>
          </div>
        )}
        {auth.status === "unavailable" && (
          <div className="flex flex-col items-start gap-3">
            <AuthError message={auth.message} />
            <Button variant="secondary" onClick={() => void identify()}>
              Try again
            </Button>
          </div>
        )}
      </section>

      {auth.status === "signed-in" && (
        <section className="flex flex-col gap-3">
          <div className="flex items-center justify-between gap-4">
            <h3 className="text-lg font-medium">Your open pull requests</h3>
            <Button
              disabled={prLoading}
              onClick={() => void loadPullRequests()}
            >
              Refresh
            </Button>
          </div>

          {prStatus !== null && (
            <p className="text-sm text-muted-foreground">{prStatus}</p>
          )}

          {pullRequests.length > 0 && (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Pull request</TableHead>
                  <TableHead>Updated</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {pullRequests.map((pr) => (
                  <PullRequestRow
                    key={`${pr.repository}#${pr.number}`}
                    pr={pr}
                    onOpenFailed={() => setPrStatus("Could not open a browser.")}
                  />
                ))}
              </TableBody>
            </Table>
          )}
        </section>
      )}
    </div>
  );
}
