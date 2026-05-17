import { useState, useEffect, useCallback } from "react";
import { getMediaInfo, mediaControl, fmtSecs } from "../api";
import type { MediaInfo } from "../types";

export function MediaWidget() {
  const [info, setInfo] = useState<MediaInfo | null>(null);
  const [noMedia, setNoMedia] = useState(false);

  const poll = useCallback(async () => {
    try {
      const m = await getMediaInfo();
      setInfo(m);
      setNoMedia(false);
    } catch (e) {
      if (String(e).includes("no_playback")) {
        setNoMedia(true);
        setInfo(null);
      }
    }
  }, []);

  useEffect(() => {
    poll();
    const id = setInterval(poll, 3_000);
    return () => clearInterval(id);
  }, [poll]);

  async function ctrl(action: string) {
    await mediaControl(action);
    setTimeout(poll, 300);
  }

  const pct = info && info.duration_secs > 0
    ? (info.position_secs / info.duration_secs) * 100
    : 0;

  return (
    <div className="spotify-widget">
      <div className="widget-header">
        <span className="widget-label">
          {info?.source_app || "Media"}
        </span>
        {info && (
          <span className="widget-status">
            {fmtSecs(info.position_secs)} / {fmtSecs(info.duration_secs)}
          </span>
        )}
      </div>

      <div className="spotify-track">
        {info?.thumbnail_b64 ? (
          <img className="spotify-art" src={info.thumbnail_b64} alt="" />
        ) : (
          <div className="spotify-art-placeholder">
            {noMedia ? "—" : "♪"}
          </div>
        )}

        <div className="spotify-meta">
          {noMedia ? (
            <div className="spotify-artist">Nothing playing</div>
          ) : info ? (
            <>
              <div className="spotify-title">{info.title || "Unknown"}</div>
              <div className="spotify-artist">{info.artist || "Unknown artist"}</div>
            </>
          ) : (
            <div className="spotify-artist">…</div>
          )}
        </div>

        <div className="spotify-controls">
          <button className="ctrl-btn" onClick={() => ctrl("previous")}>⏮</button>
          <button
            className="ctrl-btn play-pause"
            onClick={() => ctrl("play_pause")}
          >
            {info?.is_playing ? "⏸" : "▶"}
          </button>
          <button className="ctrl-btn" onClick={() => ctrl("next")}>⏭</button>
        </div>
      </div>

      <div className="progress-bar">
        <div className="progress-fill spotify" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
