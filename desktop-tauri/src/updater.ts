import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

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

/** Check GitHub Release latest.json (HTTPS + signed). Returns null if up to date. */
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
 * Download + install a verified update, then relaunch the console.
 * Does not touch `.gateway/models.json` (installer / updater leaves user data).
 */
export async function downloadInstallAndRelaunch(
  onProgress?: (p: UpdateProgress) => void,
): Promise<void> {
  onProgress?.({
    phase: "checking",
    downloaded: 0,
    total: null,
    message: "正在连接更新通道…",
  });

  const update = await check();
  if (!update) {
    throw new Error("当前已是最新版本");
  }

  let downloaded = 0;
  let total: number | null = null;

  onProgress?.({
    phase: "downloading",
    downloaded: 0,
    total: null,
    message: `正在下载 ${update.version}…`,
  });

  await update.downloadAndInstall((event: DownloadEvent) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? null;
      downloaded = 0;
      onProgress?.({
        phase: "downloading",
        downloaded,
        total,
        message: total
          ? `正在下载 ${update.version}（0 / ${formatBytes(total)}）`
          : `正在下载 ${update.version}…`,
      });
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      onProgress?.({
        phase: "downloading",
        downloaded,
        total,
        message: total
          ? `正在下载 ${update.version}（${formatBytes(downloaded)} / ${formatBytes(total)}）`
          : `正在下载 ${update.version}（${formatBytes(downloaded)}）`,
      });
    } else if (event.event === "Finished") {
      onProgress?.({
        phase: "installing",
        downloaded,
        total,
        message: "下载完成，正在安装…",
      });
    }
  });

  onProgress?.({
    phase: "done",
    downloaded,
    total,
    message: "安装完成，即将重启控制台…",
  });

  // Console relaunch only — gateway process is independent and not killed by updater.
  await relaunch();
}

/** Keep handle to an Update if caller already checked (avoids double fetch). */
export async function installKnownUpdate(
  update: Update,
  onProgress?: (p: UpdateProgress) => void,
): Promise<void> {
  let downloaded = 0;
  let total: number | null = null;
  onProgress?.({
    phase: "downloading",
    downloaded: 0,
    total: null,
    message: `正在下载 ${update.version}…`,
  });
  await update.downloadAndInstall((event: DownloadEvent) => {
    if (event.event === "Started") {
      total = event.data.contentLength ?? null;
      downloaded = 0;
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
    }
    onProgress?.({
      phase: event.event === "Finished" ? "installing" : "downloading",
      downloaded,
      total,
      message:
        event.event === "Finished"
          ? "下载完成，正在安装…"
          : total
            ? `正在下载（${formatBytes(downloaded)} / ${formatBytes(total)}）`
            : `正在下载（${formatBytes(downloaded)}）`,
    });
  });
  onProgress?.({
    phase: "done",
    downloaded,
    total,
    message: "安装完成，即将重启控制台…",
  });
  await relaunch();
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}
