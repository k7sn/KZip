import React, { useState } from "react";
import { FileArchive, X } from "lucide-react";

interface ArchiveModalProps {
  defaultArchiveName: string;
  destinationDir: string;
  selectedCount: number;
  onClose: () => void;
  onConfirmCompress: (fullArchivePath: string) => Promise<void>;
}

export const ArchiveModal: React.FC<ArchiveModalProps> = ({
  defaultArchiveName,
  destinationDir,
  selectedCount,
  onClose,
  onConfirmCompress,
}) => {
  const [archiveName, setArchiveName] = useState(defaultArchiveName);
  const [format, setFormat] = useState<".7z" | ".zip" | ".tar">(".7z");
  const [busy, setBusy] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!archiveName.trim()) return;
    setBusy(true);
    try {
      const cleanName = archiveName
        .trim()
        .replace(/\.(7z|zip|tar|rar)$/i, "");
      const sep = destinationDir.endsWith("\\") ? "" : "\\";
      const fullPath = `${destinationDir}${sep}${cleanName}${format}`;
      await onConfirmCompress(fullPath);
      onClose();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-window" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <FileArchive size={16} />
            <span>Comprimi {selectedCount} elemento/i selezionato/i</span>
          </div>
          <button className="tab-close-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>

        <form className="modal-body" onSubmit={handleSubmit}>
          <div className="form-field">
            <label>Nome Archivio</label>
            <input
              type="text"
              value={archiveName}
              onChange={(e) => setArchiveName(e.target.value)}
              autoFocus
            />
          </div>

          <div className="form-field">
            <label>Formato Archivio (KZip.Core.dll COM IOutArchive)</label>
            <select
              value={format}
              onChange={(e) =>
                setFormat(e.target.value as ".7z" | ".zip" | ".tar")
              }
            >
              <option value=".7z">7-Zip (.7z) - Massima compressione LZMA2</option>
              <option value=".zip">ZIP (.zip) - Compatibilità universale</option>
              <option value=".tar">TAR (.tar) - Archivio non compresso</option>
            </select>
          </div>

          <div className="form-field">
            <label>Cartella di destinazione</label>
            <input type="text" value={destinationDir} readOnly />
          </div>

          <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
            <button type="button" className="secondary-btn" onClick={onClose}>
              Annulla
            </button>
            <button type="submit" className="primary-btn" disabled={busy}>
              {busy ? "Compressione in corso..." : "Crea Archivio"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
