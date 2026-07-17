import { useEffect, useRef, useState } from "react";
import { getCodexUsage } from "../api";
import type { CodexUsage } from "../types";
import { useNowMs } from "../usage";
import { UsageLimitRow } from "./UsageLimitRow";

export function CodexWidget() {
  const [data, setData] = useState<CodexUsage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [spinning, setSpinning] = useState(false);
  const loadRef = useRef<() => void>(() => {});
  const nowMs = useNowMs();

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
    await Promise.all([loadRef.current(), new Promise((r) => setTimeout(r, 400))]);
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
          <UsageLimitRow
            label="5h remaining"
            usedPct={data.five_hour_remaining_pct === undefined ? undefined : 100 - data.five_hour_remaining_pct}
            resetAt={data.five_hour_resets_at}
            windowHours={5}
            nowMs={nowMs}
            color="codex"
          />
          <UsageLimitRow
            label="weekly remaining"
            usedPct={data.weekly_remaining_pct === undefined ? undefined : 100 - data.weekly_remaining_pct}
            resetAt={data.weekly_resets_at}
            windowHours={24 * 7}
            nowMs={nowMs}
            color="codex"
          />
        </div>
      ) : (
        <span className="widget-muted">loading...</span>
      )}
    </div>
  );
}
