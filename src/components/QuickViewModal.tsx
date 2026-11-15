import React from "react";
import { X, FileText } from "lucide-react";

interface QuickViewModalProps {
  title: string;
  content: string;
  onClose: () => void;
}

export const QuickViewModal: React.FC<QuickViewModalProps> = ({
  title,
  content,
  onClose,
}) => {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal-window" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <FileText size={16} />
            <span>Quick View (F3) — {title}</span>
          </div>
          <button className="tab-close-btn" onClick={onClose}>
            <X size={16} />
          </button>
        </div>
        <div className="modal-body">
          <pre className="preview-code-box">{content}</pre>
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <button className="secondary-btn" onClick={onClose}>
              Chiudi (Esc)
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
