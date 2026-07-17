import { forecastUsage, formatPace, formatReset } from "../usage";

interface Props {
  label: string;
  usedPct?: number;
  resetAt?: string;
  windowHours: number;
  nowMs: number;
  color: "claude" | "codex";
}

export function UsageLimitRow({ label, usedPct, resetAt, windowHours, nowMs, color }: Props) {
  const remaining = usedPct === undefined
    ? undefined
    : Math.round(Math.max(0, Math.min(100, 100 - usedPct)));
  const forecast = forecastUsage(usedPct, resetAt, windowHours, nowMs);
  const atRisk = forecast !== undefined && forecast.projectedPct > 100;

  return (
    <div className="usage-limit">
      <div className="widget-main">
        <span className="widget-value">{remaining === undefined ? "—" : `${remaining}%`}</span>
        <span className="widget-sub">{label}</span>
        {resetAt && <span className="widget-sub">resets {formatReset(resetAt, nowMs)}</span>}
      </div>
      <div className="progress-bar">
        <div
          className={`progress-fill ${color}`}
          style={{ width: `${remaining ?? 0}%` }}
        />
      </div>
      <div className="usage-forecast">
        {forecast ? (
          <>
            <span className={`usage-forecast-status ${atRisk ? "at-risk" : "on-track"}`}>
              {atRisk ? "at risk" : "on track"} · ~{Math.round(forecast.projectedPct)}% at reset
            </span>
            <span className="widget-sub">pace {formatPace(forecast)}</span>
          </>
        ) : (
          <span className="widget-sub">pace unavailable</span>
        )}
      </div>
    </div>
  );
}
