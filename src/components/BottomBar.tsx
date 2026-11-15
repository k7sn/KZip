import React from "react";

interface BottomBarProps {
  onViewF3: () => void;
  onEditF4: () => void;
  onCopyF5: () => void;
  onMoveF6: () => void;
  onNewFolderF7: () => void;
  onDeleteF8: () => void;
}

export const BottomBar: React.FC<BottomBarProps> = ({
  onViewF3,
  onEditF4,
  onCopyF5,
  onMoveF6,
  onNewFolderF7,
  onDeleteF8,
}) => {
  return (
    <footer className="bottom-commander-bar">
      <div className="function-keys-row">
        <button className="fn-key-btn" onClick={onViewF3}>
          F3 Anteprima / Diagnostica
        </button>
        <button className="fn-key-btn" onClick={onEditF4}>
          F4 Rinomina
        </button>
        <button className="fn-key-btn" onClick={onCopyF5}>
          F5 Copia
        </button>
        <button className="fn-key-btn" onClick={onMoveF6}>
          F6 Sposta
        </button>
        <button className="fn-key-btn" onClick={onNewFolderF7}>
          F7 Nuova Cartella
        </button>
        <button className="fn-key-btn" onClick={onDeleteF8}>
          F8 Elimina
        </button>
      </div>
    </footer>
  );
};
