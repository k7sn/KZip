import React, { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen, FolderOutput, X } from "lucide-react";
import { ExtractOptions } from "../types/vfs";

interface ExtractModalProps {
  archivePath: string;
  defaultDestinationDir: string;
  onClose: () => void;
  onConfirmExtract: (opts: ExtractOptions) => Promise<void>;
}

export const ExtractModal: React.FC<ExtractModalProps> = ({
  archivePath,
  defaultDestinationDir,
  onClose,
  onConfirmExtract,
}) => {
  const [baseDestDir, setBaseDestDir] = useState(defaultDestinationDir);
  const [createSubfolder, setCreateSubfolder] = useState(true);
  const [overwriteMode, setOverwriteMode] = useState("overwrite");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);

  const archiveStem = useMemo(() => {
    const fileName = archivePath.split("\\").pop() || "Archivio";
    return fileName.replace(/\.(7z|zip|rar|tar|gz|tgz|bz2|xz|iso|wim|cab)$/i, "");
  }, [archivePath]);

  const effectiveDestDir = useMemo(() => {
    const trimmed = baseDestDir.trim();
    if (!createSubfolder) return trimmed;
    const sep = trimmed.endsWith("\\") ? "" : "\\";
    return `${trimmed}${sep}${archiveStem}`;
  }, [baseDestDir, createSubfolder, archiveStem]);

  const handleBrowseFolder = async () => {
    try {
      const picked = await invoke<string | null>("pick_folder_native", {
        initialDir: baseDestDir,
      });
      if (picked) {
        setBaseDestDir(picked);
      }
    } catch (err) {
      alert(String(err));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!effectiveDestDir) return;
    const opts: ExtractOptions = {
      archivePath,
      destinationDir: effectiveDestDir,
      overwriteMode,
      password: password ? password : null,
    };
    onClose();
    await onConfirmExtract(opts);
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-window" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <FolderOutput size={16} color="var(--accent-color)" />
            <span>Estrai archivio — {archivePath.split("\\").pop()}</span>
          </div>
          <button className="tab-close-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <form className="modal-body" onSubmit={handleSubmit}>
          <div className="form-field">
            <label>Estrai in:</label>
            <div style={{ display: "flex", gap: 6 }}>
              <input
                type="text"
                style={{ flex: 1 }}
                value={baseDestDir}
                onChange={(e) => setBaseDestDir(e.target.value)}
                autoFocus
              />
              <button
                type="button"
                className="secondary-btn"
                onClick={handleBrowseFolder}
                style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
              >
                <FolderOpen size={14} />
                <span>Sfoglia...</span>
              </button>
            </div>
          </div>

          <label className="sz-checkbox">
            <input
              type="checkbox"
              checked={createSubfolder}
              onChange={(e) => setCreateSubfolder(e.target.checked)}
            />
            <span>
              Estrai nella sottocartella <strong>"{archiveStem}\"</strong>
            </span>
          </label>

          <div
            style={{
              fontSize: 11.5,
              padding: "6px 10px",
              borderRadius: 6,
              background: "var(--sub-bar-bg)",
              color: "var(--text-secondary)",
            }}
          >
            Percorso finale: <strong>{effectiveDestDir}</strong>
          </div>

          <div className="form-field">
            <label>Modalità di sovrascrittura:</label>
            <select
              value={overwriteMode}
              onChange={(e) => setOverwriteMode(e.target.value)}
            >
              <option value="overwrite">Sovrascrivi senza chiedere</option>
              <option value="skip">Salta i file esistenti</option>
              <option value="rename">Rinomina automaticamente</option>
            </select>
          </div>

          <fieldset className="sz-groupbox">
            <legend>Password (se l'archivio è protetto)</legend>
            <div className="form-field">
              <input
                type={showPassword ? "text" : "password"}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder="Inserisci password archivio (opzionale)"
              />
            </div>
            <label className="sz-checkbox" style={{ marginTop: 4 }}>
              <input
                type="checkbox"
                checked={showPassword}
                onChange={(e) => setShowPassword(e.target.checked)}
              />
              <span>Mostra password</span>
            </label>
          </fieldset>

          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button type="button" className="secondary-btn" onClick={onClose}>
              Annulla
            </button>
            <button type="submit" className="primary-btn">
              Estrai Ora
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
