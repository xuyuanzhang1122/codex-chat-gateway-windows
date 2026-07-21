import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { FolderOpen, Gauge, LogOut, Play, Square } from "lucide-react";
import { api } from "./api";
import type { GatewayStatus } from "./types";
import "./tray-menu.css";

const initialStatus: GatewayStatus = {
  phase: "stopped",
  running: false,
  healthy: false,
  is_our_gateway: false,
  endpoint: "http://127.0.0.1:4000/v1",
  pid: null,
  model: null,
  started_at: null,
  uptime: null,
  default_model_name: null,
  message: "读取状态中…",
  routes: [],
  busy: false,
  startup_progress: null,
  startup_stage: null,
};

const hideMenu = () => invoke("hide_tray_menu");

export function TrayMenu() {
  const [status, setStatus] = useState(initialStatus);

  useEffect(() => {
    const blockContextMenu = (event: MouseEvent) => event.preventDefault();
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") void hideMenu();
    };
    window.addEventListener("contextmenu", blockContextMenu);
    window.addEventListener("keydown", escape);

    let unlisten: (() => void) | undefined;
    void api.status().then(setStatus).catch(() => undefined);
    void listen<GatewayStatus>("gateway://status", (event) => setStatus(event.payload)).then(
      (dispose) => { unlisten = dispose; },
    );

    return () => {
      unlisten?.();
      window.removeEventListener("contextmenu", blockContextMenu);
      window.removeEventListener("keydown", escape);
    };
  }, []);

  const online = status.healthy || status.running;
  const phaseLabel = status.busy
    ? status.phase === "stopping" ? "正在停止" : "正在启动"
    : status.healthy ? "运行正常"
      : status.running ? "进程在线"
        : "已停止";
  const model = status.default_model_name || status.model || "尚未设置默认上游";

  return (
    <main className="tray-popover">
      <header className="tray-popover-head">
        <img src="/gateway-logo.png" alt="" />
        <div className="tray-popover-title">
          <strong>CCG STUDIO</strong>
          <span>{model}</span>
        </div>
        <span className={`tray-phase${online ? " is-online" : ""}`}>
          <i />{phaseLabel}
        </span>
      </header>

      <section className="tray-popover-actions">
        <button type="button" onClick={() => void invoke("show_main_window")}>
          <span className="tray-action-icon"><Gauge size={17} /></span>
          <span><strong>打开控制台</strong><small>模型、分流与客户端设置</small></span>
          <b>↗</b>
        </button>
        <button
          type="button"
          className={online ? "is-stop" : "is-start"}
          disabled={status.busy}
          onClick={() => {
            void (online ? api.stop() : api.start()).finally(() => hideMenu());
          }}
        >
          <span className="tray-action-icon">{online ? <Square size={15} /> : <Play size={16} />}</span>
          <span>
            <strong>{online ? "停止网关" : "启动网关"}</strong>
            <small>{online ? `127.0.0.1:4000 · PID ${status.pid ?? "—"}` : "仅在本机启动模型桥"}</small>
          </span>
          <b>{online ? "■" : "▶"}</b>
        </button>
        <button type="button" onClick={() => void api.openLogsDir().finally(() => hideMenu())}>
          <span className="tray-action-icon"><FolderOpen size={16} /></span>
          <span><strong>打开日志目录</strong><small>查看启动与请求诊断</small></span>
          <b>↗</b>
        </button>
      </section>

      <footer className="tray-popover-foot">
        <span>关闭控制台不会停止网关</span>
        <button type="button" onClick={() => void invoke("quit_console")}>
          <LogOut size={13} />退出控制台
        </button>
      </footer>
    </main>
  );
}
