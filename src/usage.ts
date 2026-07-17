import { useEffect, useState } from "react";

const DAY_MS = 24 * 60 * 60 * 1000;

export interface UsageForecast {
  projectedPct: number;
  pacePct: number;
  paceUnit: "hour" | "day";
}

/** Keeps countdowns current without polling either provider more often. */
export function useNowMs() {
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    const id = window.setInterval(() => setNowMs(Date.now()), 30_000);
    return () => window.clearInterval(id);
  }, []);

  return nowMs;
}

function formatDuration(seconds: number) {
  if (seconds < 60) return `${seconds}s`;

  const minutes = Math.ceil(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;

  if (hours === 0) return `${minutes}m`;
  if (remainingMinutes === 0) return `${hours}h`;
  return `${hours}h ${remainingMinutes}m`;
}

export function formatReset(resetAt?: string, nowMs = Date.now()) {
  if (!resetAt) return undefined;

  const resetMs = Date.parse(resetAt);
  if (Number.isNaN(resetMs)) return resetAt;

  const seconds = Math.max(0, Math.ceil((resetMs - nowMs) / 1000));
  if (seconds === 0) return "now";
  if (seconds <= DAY_MS / 1000) return `in ${formatDuration(seconds)}`;

  return new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(resetMs));
}

export function forecastUsage(
  usedPct: number | undefined,
  resetAt: string | undefined,
  windowHours: number,
  nowMs: number,
): UsageForecast | undefined {
  if (usedPct === undefined || !resetAt) return undefined;

  const resetMs = Date.parse(resetAt);
  if (Number.isNaN(resetMs) || resetMs <= nowMs) return undefined;

  const windowMs = windowHours * 60 * 60 * 1000;
  const elapsedMs = Math.min(windowMs, Math.max(0, nowMs - (resetMs - windowMs)));
  if (elapsedMs <= 0) return undefined;

  const elapsedHours = elapsedMs / (60 * 60 * 1000);
  return {
    projectedPct: Math.max(0, usedPct) * (windowMs / elapsedMs),
    pacePct: Math.max(0, usedPct) / elapsedHours,
    paceUnit: windowHours <= 24 ? "hour" : "day",
  };
}

export function formatPace(forecast: UsageForecast) {
  const value = forecast.paceUnit === "day" ? forecast.pacePct * 24 : forecast.pacePct;
  const rounded = value >= 10 ? Math.round(value) : Math.round(value * 10) / 10;
  return `${rounded}%/${forecast.paceUnit === "day" ? "day" : "h"}`;
}
