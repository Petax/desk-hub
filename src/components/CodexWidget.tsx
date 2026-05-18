import { useEffect, useRef, useState } from "react";
import { getCodexUsage } from "../api";
import type { CodexUsage } from "../types";

function pct(value?: number) {
  return Math.round(Math.max(0, Math.min(100, value ?? 0)));
}

function LimitRow({
  label,
  remaining,
  resetsAt,
}: {
  label: string;
  remaining?: number;
  resetsAt?: string;
}) {
  const rounded = pct(remaining);

  return (
    <div className="usage-limit">
      <div className="widget-main">
        <span className="widget-value">{rounded}%</span>
        <span className="widget-sub">{label}</span>
        {resetsAt && <span className="widget-sub">resets {resetsAt}</span>}
      </div>
      <div className="progress-bar">
        <div className="progress-fill codex" style={{ width: `${rounded}%` }} />
      </div>
    </div>
  );
}

export function CodexWidget() {
  const [data, setData] = useState<CodexUsage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [spinning, setSpinning] = useState(false);
  const loadRef = useRef<() => void>(() => {});

  useEffect(() => {
    let cancelled = false;

    async function load() {
      try {
        const usage = await getCodexUsage();
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
    const id = setInterval(load, 120_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  async function handleRefresh() {
    setSpinning(true);
    await loadRef.current();
    setSpinning(false);
  }

  return (
    <div className="widget codex-widget">
      <div className="widget-header">
        <span className="widget-label">
          Codex{data?.plan ? ` - ${data.plan}` : ""}
        </span>
        {data && <span className="widget-status">live</span>}
        <button className={`btn-icon refresh-btn${spinning ? " spinning" : ""}`} title="Refresh" onClick={handleRefresh}>↻</button>
      </div>

      {error ? (
        <span className="widget-error">{error}</span>
      ) : data ? (
        <div className="usage-limits">
          <LimitRow
            label="5h remaining"
            remaining={data.five_hour_remaining_pct}
            resetsAt={data.five_hour_resets_at}
          />
          <LimitRow
            label="weekly remaining"
            remaining={data.weekly_remaining_pct}
            resetsAt={data.weekly_resets_at}
          />
        </div>
      ) : (
        <span className="widget-muted">loading...</span>
      )}
    </div>
  );
}
