import { check, type Update } from "@tauri-apps/plugin-updater";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type UpdateInfo = {
  version: string;
  currentVersion: string;
  date?: string;
  body?: string;
};

export type UpdateProgress = {
  phase: "checking" | "downloading" | "installing" | "done" | "idle";
  downloaded: number;
  total: number | null;
  message: string;
};

type BackendProgress = {
  downloaded: number;
  total: number | null;
  verified?: boolean;
};

/** Check GitHub Release latest.json (HTTPS). Returns null if up to date. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  const update = await check();
  if (!update) return null;
  return {
    version: update.version,
    currentVersion: update.currentVersion,
    date: update.date,
    body: update.body,
  };
}

/**
 * Download the full Studio (Inno) installer — SHA-256 verified by the Rust
 * side — then launch it and let the console exit. The Inno wizard upgrades
 * in place and preserves runtime/, scripts/ and .gateway/.
 *
 * We deliberately do NOT use the Tauri NSIS updater package: it bundles only
 * the bare console exe and installs to a different directory, which produced
 * broken half-installs (no gateway runtime, endless update prompts).
 */
export async function installKnownUpdate(
  update: Update,
  onProgress?: (p: UpdateProgress) => void,
): Promise<void> {
  const version = update.version;
  onProgress?.({
    phase: "downloading",
    downloaded: 0,
    total: null,
    message: `正在下载 ${version} 完整安装包…`,
  });

  const unlisten = await listen<BackendProgress>("update://progress", (e) => {
    const { downloaded, total, verified } = e.payload;
    onProgress?.({
      phase: verified ? "installing" : "downloading",
      downloaded,
      total,
      message: verified
        ? "SHA-256 校验完成，正在启动安装器…"
        : total
          ? `正在下载 ${version}（${formatBytes(downloaded)} / ${formatBytes(total)}）`
          : `正在下载 ${version}（${formatBytes(downloaded)}）`,
    });
  });

  try {
    // Resolves right before the console exits for the installer.
    await invoke("download_studio_installer", { version });
    onProgress?.({
      phase: "done",
      downloaded: 0,
      total: null,
      message: "安装器已启动，控制台即将退出…",
    });
  } finally {
    unlisten();
  }
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}
