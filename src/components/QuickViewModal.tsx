import React from "react";
import { FileText, RefreshCw, X } from "lucide-react";

interface QuickViewModalProps {
  title: string;
  content: string;
  onReloadDll?: () => void;
  onClose: () => void;
}

export const QuickViewModal: React.FC<QuickViewModalProps> = ({
  title,
  content,
  onReloadDll,
  onClose,
}) => {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-window" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <FileText size={16} />
            <span>{title}</span>
          </div>
          <button className="tab-close-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>
        <div className="modal-body">
          <pre className="preview-code-box">{content}</pre>
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
            }}
          >
            {onReloadDll ? (
              <button
                type="button"
                className="secondary-btn"
                onClick={onReloadDll}
                style={{ display: "inline-flex", alignItems: "center", gap: 6 }}
              >
                <RefreshCw size={14} />
                <span>Riprova caricamento KZip.Core.dll</span>
              </button>
            ) : (
              <span />
            )}
            <button className="primary-btn" onClick={onClose}>
              Chiudi (Esc)
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
