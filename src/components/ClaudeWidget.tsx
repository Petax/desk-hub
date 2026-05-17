import { useState, useEffect } from "react";
import { getClaudeUsage } from "../api";
import type { AppConfig, ClaudeUsage } from "../types";

interface Props {
  config: AppConfig;
}

function Bar({ pct, cls }: { pct: number; cls: string }) {
  return (
    <div className="progress-bar">
      <div className={`progress-fill ${cls}`} style={{ width: `${Math.min(pct, 100)}%` }} />
    </div>
  );
}

export function ClaudeWidget({ config }: Props) {
  const [data, setData] = useState<ClaudeUsage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const u = await getClaudeUsage();
        if (!cancelled) { setData(u); setError(null); }
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    }
    load();
    const id = setInterval(load, 60_000);
    return () => { cancelled = true; clearInterval(id); };
  }, [config.claude_session_key]);

  return (
    <div className="widget">
      <div className="widget-header">
        <span className="widget-label">
          Claude {data?.plan ? `· ${data.plan}` : ""}
        </span>
        {!config.claude_session_key && (
          <span className="widget-status">local</span>
        )}
      </div>

      {error ? (
        <span className="widget-error">{error}</span>
      ) : !data ? (
        <span className="widget-muted">loading…</span>
      ) : data.session_pct !== undefined ? (
        // API mode — show session + weekly like the claude.ai page
        <>
          <div className="widget-main" style={{ marginBottom: 3 }}>
            <span className="widget-value">{Math.round(data.session_pct)}%</span>
            <span className="widget-sub">session used</span>
            {data.session_resets_in && (
              <span className="widget-sub">· resets {data.session_resets_in}</span>
            )}
          </div>
          <Bar pct={data.session_pct} cls="claude" />
          {data.weekly_pct !== undefined && (
            <>
              <div className="widget-main" style={{ marginTop: 5, marginBottom: 3 }}>
                <span className="widget-value">{Math.round(data.weekly_pct)}%</span>
                <span className="widget-sub">weekly</span>
                {data.weekly_resets_at && (
                  <span className="widget-sub">· resets {data.weekly_resets_at}</span>
                )}
              </div>
              <Bar pct={data.weekly_pct} cls="claude" />
            </>
          )}
        </>
      ) : (
        // Fallback: local message count
        <>
          <div className="widget-main">
            <span className="widget-value">
              {data.local_messages ?? 0} / {data.local_limit}
            </span>
            <span className="widget-sub">msgs today</span>
          </div>
          <Bar pct={((data.local_messages ?? 0) / data.local_limit) * 100} cls="claude" />
          <div className="widget-hint">Add session key in ⚙ for live %</div>
        </>
      )}
    </div>
  );
}
