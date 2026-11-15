import React, { useState } from "react";
import { ChevronDown } from "lucide-react";

interface BottomBarProps {
  activePath: string;
  onExecutePathOrCommand: (input: string) => void;
  onViewF3: () => void;
  onEditF4: () => void;
  onCopyF5: () => void;
  onMoveF6: () => void;
  onNewFolderF7: () => void;
  onDeleteF8: () => void;
}

export const BottomBar: React.FC<BottomBarProps> = ({
  activePath,
  onExecutePathOrCommand,
  onViewF3,
  onEditF4,
  onCopyF5,
  onMoveF6,
  onNewFolderF7,
  onDeleteF8,
}) => {
  const [cmdInput, setCmdInput] = useState("");

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && cmdInput.trim()) {
      onExecutePathOrCommand(cmdInput.trim());
      setCmdInput("");
    }
  };

  return (
    <footer className="bottom-commander-bar">
      <div className="command-line-row">
        <div className="command-prompt-path" title={activePath}>
          {activePath}
        </div>
        <div className="command-input-wrap">
          <input
            type="text"
            value={cmdInput}
            onChange={(e) => setCmdInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Digita un percorso (es. C:\Windows) o premi Invio per navigare..."
          />
          <button
            className="cmd-dropdown-btn"
            onClick={() => {
              if (cmdInput.trim()) {
                onExecutePathOrCommand(cmdInput.trim());
                setCmdInput("");
              }
            }}
            title="Vai al percorso"
          >
            <ChevronDown size={13} />
          </button>
        </div>
      </div>

      <div className="function-keys-row">
        <button className="fn-key-btn" onClick={onViewF3}>
          View - F3
        </button>
        <button className="fn-key-btn" onClick={onEditF4}>
          Edit - F4
        </button>
        <button className="fn-key-btn" onClick={onCopyF5}>
          Copy - F5
        </button>
        <button className="fn-key-btn" onClick={onMoveF6}>
          Move - F6
        </button>
        <button className="fn-key-btn" onClick={onNewFolderF7}>
          New Folder - F7
        </button>
        <button className="fn-key-btn" onClick={onDeleteF8}>
          Delete - F8
        </button>
      </div>
    </footer>
  );
};
