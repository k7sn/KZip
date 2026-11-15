import React from "react";
import { FileArchive, Maximize2, Minimize2, XCircle } from "lucide-react";
import { ProgressPayload } from "../types/vfs";

interface ProgressModalProps {
  progress: ProgressPayload;
  minimized: boolean;
  onToggleMinimize: () => void;
  onCancel: () => void;
  onCloseFinished: () => void;
}

export const ProgressModal: React.FC<ProgressModalProps> = ({
  progress,
  minimized,
  onToggleMinimize,
  onCancel,
  onCloseFinished,
}) => {
  const isRunning = progress.status === "running";
  const pct = Math.min(100, Math.max(0, progress.percent));

  if (minimized) {
    return (
      <div className="progress-floating-pill" onClick={onToggleMinimize}>
        <FileArchive size={15} color="var(--accent-color)" />
        <span>
          {progress.operation}: <strong>{pct.toFixed(0)}%</strong> (
          {progress.speedMbPerSec.toFixed(1)} MB/s)
        </span>
        <Maximize2 size={13} />
      </div>
    );
  }

  return (
    <div className="modal-backdrop">
      <div className="modal-window" style={{ width: 520 }}>
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <FileArchive size={16} color="var(--accent-color)" />
            <span>
              {progress.operation} — {progress.archiveName} (
              {pct.toFixed(1)}%)
            </span>
          </div>
          <button
            className="tab-close-btn"
            onClick={onToggleMinimize}
            title="Minimizza in background"
          >
            <Minimize2 size={15} />
          </button>
        </div>

        <div className="modal-body">
          <div style={{ fontSize: 12, color: "var(--text-secondary)" }}>
            File in elaborazione:
            <div
              style={{
                fontWeight: 600,
                color: "var(--text-primary)",
                marginTop: 3,
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              }}
            >
              {progress.currentFile || "Preparazione..."}
            </div>
          </div>

          {/* Barra di progresso fluida */}
          <div className="progress-track">
            <div
              className="progress-fill"
              style={{ width: `${pct}%` }}
            />
          </div>

          <div className="progress-stats-grid">
            <div>
              <span>Completato:</span>{" "}
              <strong>{pct.toFixed(1)}%</strong>
            </div>
            <div>
              <span>Velocità:</span>{" "}
              <strong>{progress.speedMbPerSec.toFixed(2)} MB/s</strong>
            </div>
            <div>
              <span>Dati elaborati:</span>{" "}
              <strong>
                {formatBytes(progress.processedBytes)} /{" "}
                {formatBytes(progress.totalBytes)}
              </strong>
            </div>
            <div>
              <span>Tempo trascorso / stimato:</span>{" "}
              <strong>
                {formatSecs(progress.elapsedSecs)} /{" "}
                {isRunning ? formatSecs(progress.etaSecs) : "00:00"}
              </strong>
            </div>
          </div>

          {progress.message && (
            <div
              style={{
                padding: "8px 10px",
                borderRadius: 6,
                background: "rgba(220, 38, 38, 0.12)",
                color: "#ef4444",
                fontSize: 12,
              }}
            >
              {progress.message}
            </div>
          )}

          <div
            style={{
              display: "flex",
              justifyContent: "flex-end",
              gap: 8,
              marginTop: 4,
            }}
          >
            {isRunning ? (
              <>
                <button
                  type="button"
                  className="secondary-btn"
                  onClick={onToggleMinimize}
                >
                  Sfondo
                </button>
                <button
                  type="button"
                  className="secondary-btn"
                  onClick={onCancel}
                  style={{
                    display: "inline-flex",
                    alignItems: "center",
                    gap: 6,
                    color: "#dc2626",
                  }}
                >
                  <XCircle size={14} />
                  <span>Annulla</span>
                </button>
              </>
            ) : (
              <button
                type="button"
                className="primary-btn"
                onClick={onCloseFinished}
              >
                Chiudi
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

function formatBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  const kb = b / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  return `${(mb / 1024).toFixed(2)} GB`;
}

function formatSecs(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
