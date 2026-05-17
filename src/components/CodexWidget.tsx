import { useState, useEffect } from "react";
import { getOpenAIUsage, fmtTokens } from "../api";
import type { AppConfig, OpenAIUsage } from "../types";

interface Props {
  config: AppConfig;
}

export function CodexWidget({ config }: Props) {
  const [data, setData] = useState<OpenAIUsage | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!config.openai_api_key) { setError("no_key"); return; }
    let cancelled = false;
    async function load() {
      try {
        const u = await getOpenAIUsage(config.openai_api_key!);
        if (!cancelled) { setData(u); setError(null); }
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    }
    load();
    const id = setInterval(load, 120_000);
    return () => { cancelled = true; clearInterval(id); };
  }, [config.openai_api_key]);

  return (
    <div className="widget">
      <div className="widget-header">
        <span className="widget-label">OpenAI / Codex</span>
        {data && <span className="widget-status">today</span>}
      </div>
      {error === "no_key" ? (
        <span className="widget-muted">Add API key in ⚙</span>
      ) : error ? (
        <span className="widget-error">{error}</span>
      ) : data ? (
        <div className="widget-main">
          <span className="widget-value">{fmtTokens(data.tokens_used)}</span>
          <span className="widget-sub">tokens · {data.requests} req</span>
        </div>
      ) : (
        <span className="widget-muted">loading…</span>
      )}
    </div>
  );
}
