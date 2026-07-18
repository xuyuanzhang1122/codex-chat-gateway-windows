import { Text } from "@lobehub/ui";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function TitleBar() {
  const win = getCurrentWindow();

  return (
    <header className="titlebar">
      <div
        className="titlebar-drag"
        data-tauri-drag-region
        onDoubleClick={() => void win.toggleMaximize()}
      >
        <img className="titlebar-logo" src="/gateway-logo.png" alt="" draggable={false} />
        <Text fontSize={12} type="secondary" data-tauri-drag-region>
          Codex Chat Gateway
        </Text>
      </div>
      <div className="win-controls">
        <button type="button" className="win-btn" aria-label="Minimize" onClick={() => void win.minimize()}>
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.2">
            <path d="M1 5h8" />
          </svg>
        </button>
        <button type="button" className="win-btn" aria-label="Maximize" onClick={() => void win.toggleMaximize()}>
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.2">
            <rect x="1.5" y="1.5" width="7" height="7" rx="0.5" />
          </svg>
        </button>
        <button type="button" className="win-btn close" aria-label="Close" onClick={() => void win.close()}>
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.2">
            <path d="M2 2l6 6M8 2l-6 6" />
          </svg>
        </button>
      </div>
    </header>
  );
}
