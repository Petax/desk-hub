import { useEffect, useRef, useState } from "react";
import { getClaudeUsage } from "../api";
import type { AppConfig, ClaudeUsage } from "../types";
import { useNowMs } from "../usage";
import { UsageLimitRow } from "./UsageLimitRow";

interface Props {
  config: AppConfig;
}

export function ClaudeWidget({ config }: Props) {
  const [data, setData] = useState<ClaudeUsage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [spinning, setSpinning] = useState(false);
  const loadRef = useRef<() => void>(() => {});
  const nowMs = useNowMs();

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const usage = await getClaudeUsage();
        if (!cancelled) {
          setData(usage);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    }

    loadRef.current = load;
    load();
    const id = setInterval(load, 300_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [config.claude_session_key]);

  async function handleRefresh() {
    setSpinning(true);
    await loadRef.current();
    setSpinning(false);
  }

  return (
    <div className="widget">
      <div className="widget-header">
        <span className="widget-label">
          Claude{data?.plan ? ` - ${data.plan}` : ""}
        </span>
        <span className="widget-status">{config.claude_session_key ? "live" : "local"}</span>
        <button className={`btn-icon refresh-btn${spinning ? " spinning" : ""}`} title="Refresh" onClick={handleRefresh}>↻</button>
      </div>

      {error ? (
        <span className="widget-error">{error}</span>
      ) : !data ? (
        <span className="widget-muted">loading...</span>
      ) : data.session_pct !== undefined ? (
        <div className="usage-limits">
          <UsageLimitRow
            label="5h remaining"
            usedPct={data.session_pct}
            resetAt={data.session_resets_at}
            windowHours={5}
            nowMs={nowMs}
            color="claude"
          />
          {data.weekly_pct !== undefined && (
            <UsageLimitRow
              label="weekly remaining"
              usedPct={data.weekly_pct}
              resetAt={data.weekly_resets_at}
              windowHours={24 * 7}
              nowMs={nowMs}
              color="claude"
            />
          )}
        </div>
      ) : (
        <>
          <div className="widget-main">
            <span className="widget-value">
              {data.local_messages ?? 0} / {data.local_limit}
            </span>
            <span className="widget-sub">msgs today</span>
          </div>
          <div className="progress-bar">
            <div
              className="progress-fill claude"
              style={{
                width: `${Math.min(
                  ((data.local_messages ?? 0) / data.local_limit) * 100,
                  100,
                )}%`,
              }}
            />
          </div>
          <div className="widget-hint">Add session key in settings for live limits</div>
        </>
      )}
    </div>
  );
}
