import { useEffect, useRef, useState } from "react";
import { getClaudeUsage } from "../api";
import type { AppConfig, ClaudeUsage } from "../types";

interface Props {
  config: AppConfig;
}

function remainingPct(used?: number) {
  return Math.round(Math.max(0, Math.min(100, 100 - (used ?? 0))));
}

function LimitRow({
  label,
  used,
  resets,
}: {
  label: string;
  used?: number;
  resets?: string;
}) {
  const remaining = remainingPct(used);

  return (
    <div className="usage-limit">
      <div className="widget-main">
        <span className="widget-value">{remaining}%</span>
        <span className="widget-sub">{label}</span>
        {resets && <span className="widget-sub">resets {resets}</span>}
      </div>
      <div className="progress-bar">
        <div className="progress-fill claude" style={{ width: `${remaining}%` }} />
      </div>
    </div>
  );
}

export function ClaudeWidget({ config }: Props) {
  const [data, setData] = useState<ClaudeUsage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [spinning, setSpinning] = useState(false);
  const loadRef = useRef<() => void>(() => {});

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
          <LimitRow
            label="5h remaining"
            used={data.session_pct}
            resets={data.session_resets_in}
          />
          {data.weekly_pct !== undefined && (
            <LimitRow
              label="weekly remaining"
              used={data.weekly_pct}
              resets={data.weekly_resets_at}
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
