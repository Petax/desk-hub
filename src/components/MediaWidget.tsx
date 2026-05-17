import { useState, useEffect, useCallback, useRef } from "react";
import { getMediaInfo, mediaControl, fmtSecs } from "../api";
import type { MediaInfo } from "../types";

export function MediaWidget() {
  const [info, setInfo] = useState<MediaInfo | null>(null);
  const [receivedAt, setReceivedAt] = useState(Date.now());
  const [now, setNow] = useState(Date.now());
  const [noMedia, setNoMedia] = useState(false);
  const polling = useRef(false);

  const poll = useCallback(async () => {
    if (polling.current) return;
    polling.current = true;
    try {
      const m = await getMediaInfo();
      setInfo(m);
      setReceivedAt(Date.now());
      setNoMedia(false);
    } catch (e) {
      if (String(e).includes("no_playback")) {
        setNoMedia(true);
        setInfo(null);
      }
    } finally {
      polling.current = false;
    }
  }, []);

  useEffect(() => {
    poll();
    const id = setInterval(poll, 1_500);
    return () => clearInterval(id);
  }, [poll]);

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(id);
  }, []);

  async function ctrl(action: string) {
    await mediaControl(action);
    setTimeout(poll, 120);
    setTimeout(poll, 600);
  }

  const position = info
    ? Math.min(
        info.duration_secs || Number.POSITIVE_INFINITY,
        info.position_secs + (info.is_playing ? (now - receivedAt) / 1000 : 0),
      )
    : 0;
  const pct = info && info.duration_secs > 0
    ? (position / info.duration_secs) * 100
    : 0;

  return (
    <div className="spotify-widget">
      <div className="spotify-track">
        {info?.thumbnail_b64 ? (
          <img className="spotify-art" src={info.thumbnail_b64} alt="" />
        ) : (
          <div className="spotify-art-placeholder">
            {noMedia ? "-" : "♪"}
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
            <div className="spotify-artist">...</div>
          )}
        </div>

        {info && (
          <div className="spotify-time">
            {fmtSecs(position)} / {fmtSecs(info.duration_secs)}
          </div>
        )}
      </div>

      <div className="spotify-controls">
        <button className="ctrl-btn" title="Previous" onClick={() => ctrl("previous")}>
          ⏮
        </button>
        <button
          className="ctrl-btn play-pause"
          title={info?.is_playing ? "Pause" : "Play"}
          onClick={() => ctrl("play_pause")}
        >
          {info?.is_playing ? "⏸" : "▶"}
        </button>
        <button className="ctrl-btn" title="Next" onClick={() => ctrl("next")}>
          ⏭
        </button>
      </div>

      <div className="progress-bar">
        <div className="progress-fill spotify" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
