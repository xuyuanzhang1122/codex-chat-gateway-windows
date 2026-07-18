import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { Particles } from "./Particles";
import type {
  ActionResult,
  GatewayStatus,
  LogLevel,
  LogLine,
  ModelInput,
  ModelProfile,
  ModelStore,
  ProjectInfo,
} from "./types";
import logo from "./assets/gateway-logo.png";

const emptyStatus: GatewayStatus = {
  running: false,
  healthy: false,
  is_our_gateway: false,
  endpoint: "http://127.0.0.1:4000/v1",
  pid: null,
  model: null,
  started_at: null,
  uptime: null,
  default_model_name: null,
  message: "正在检测…",
  routes: [],
};

function safeClipboard(text: string): boolean {
  try {
    void navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const ta = document.createElement("textarea");
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      return true;
    } catch {
      return false;
    }
  }
}

export default function App() {
  const [booted, setBooted] = useState(false);
  const [bootExit, setBootExit] = useState(false);
  const [status, setStatus] = useState<GatewayStatus>(emptyStatus);
  const [store, setStore] = useState<ModelStore>({ version: 1, default_id: "", profiles: [] });
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const logId = useRef(0);
  const consoleRef = useRef<HTMLDivElement | null>(null);
  const [modelDialog, setModelDialog] = useState<null | { mode: "add" | "edit"; profile?: ModelProfile }>(null);

  const log = useCallback((level: LogLevel, messageText: string) => {
    if (!messageText) return;
    setLogs((prev) => {
      const next = [
        ...prev,
        { id: ++logId.current, level, message: messageText },
      ];
      return next.slice(-500);
    });
  }, []);

  useEffect(() => {
    consoleRef.current?.scrollTo({ top: consoleRef.current.scrollHeight });
  }, [logs]);

  const refresh = useCallback(async () => {
    try {
      const [st, models, proj] = await Promise.all([
        api.status(),
        api.listModels(),
        api.projectInfo(),
      ]);
      setStatus(st);
      setStore(models);
      setInfo(proj);
    } catch (e) {
      log("ERR", `刷新失败: ${String(e)}`);
    }
  }, [log]);

  useEffect(() => {
    if (!booted) return;
    void refresh().then(() => log("INFO", `桌面控制台已就绪。项目目录：${info?.root ?? "…"}`));
    const t = window.setInterval(() => {
      if (!busy) void api.status().then(setStatus).catch(() => undefined);
    }, 3000);
    return () => window.clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [booted]);

  const enter = () => {
    setBootExit(true);
    window.setTimeout(() => {
      setBooted(true);
      void (async () => {
        await refresh();
        const proj = await api.projectInfo().catch(() => null);
        log("INFO", `桌面控制台已就绪。项目目录：${proj?.root ?? "未知"}`);
        log("DIM", "视觉风格参考 Mineradio · 引擎 Tauri 2");
      })();
    }, 520);
  };

  const applyResult = (label: string, result: ActionResult) => {
    for (const line of result.logs) {
      log(result.ok ? "DIM" : "ERR", line);
    }
    log(result.ok ? "OK" : "ERR", result.ok ? `${label} 完成。` : `${label} 失败：${result.message}`);
    setStatus(result.status);
  };

  const withBusy = async (label: string, fn: () => Promise<ActionResult | void>) => {
    if (busy) return;
    setBusy(true);
    log("INFO", `▶ ${label}`);
    try {
      const result = await fn();
      if (result) applyResult(label, result);
      await refresh();
    } catch (e) {
      log("ERR", `${label} 异常：${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const onStart = () =>
    void (async () => {
      if (!store.profiles.some((p) => p.id === store.default_id)) {
        log("DIM", "启动前需要先添加默认模型。");
        setModelDialog({ mode: "add" });
        return;
      }
      await withBusy("启动网关", () => api.start());
    })();

  const onStop = () => void withBusy("停止网关", () => api.stop());
  const onRestart = () =>
    void (async () => {
      if (!store.profiles.some((p) => p.id === store.default_id)) {
        log("DIM", "重启前需要先配置默认模型。");
        setModelDialog({ mode: "add" });
        return;
      }
      await withBusy("重启网关", () => api.restart());
    })();

  const onCheck = () => void withBusy("接口检查", () => api.check());

  const onOpenLogs = async () => {
    try {
      const dir = await api.logsDir();
      await openPath(dir);
      log("OK", `已打开日志目录：${dir}`);
    } catch (e) {
      log("ERR", `打开日志失败：${String(e)}`);
    }
  };

  const onOpenUi = () => {
    void openUrl("http://127.0.0.1:4000/ui").catch((e) => log("ERR", String(e)));
  };

  const onCopyEndpoint = () => {
    if (safeClipboard(status.endpoint || "http://127.0.0.1:4000/v1")) {
      log("OK", "已复制接口地址");
    } else {
      log("ERR", "复制失败：剪贴板不可用");
    }
  };

  const runClientScript = async (label: string, script: string, confirmText?: string) => {
    if (confirmText) {
      const ok = await ask(confirmText, {
        title: label,
        kind: "warning",
        okLabel: "继续",
        cancelLabel: "取消",
      });
      if (!ok) {
        log("DIM", `已取消：${label}`);
        return;
      }
    }
    await withBusy(label, async () => {
      const result = await api.runScript(script);
      return result;
    });
  };

  const onSetDefault = async () => {
    if (!selectedId) {
      log("DIM", "请先选择一个模型。");
      return;
    }
    try {
      const next = await api.makeDefault(selectedId);
      setStore(next);
      log("OK", "已切换默认模型。");
      if (status.running) {
        const yes = await ask("默认模型已更改。是否立即重启网关以生效？", {
          title: "重启网关",
          kind: "info",
          okLabel: "立即重启",
          cancelLabel: "稍后",
        });
        if (yes) await withBusy("重启网关", () => api.restart());
        else log("DIM", "稍后请手动重启网关使配置生效。");
      }
    } catch (e) {
      log("ERR", String(e));
    }
  };

  const onDelete = async () => {
    if (!selectedId) {
      log("DIM", "请先选择一个模型。");
      return;
    }
    const profile = store.profiles.find((p) => p.id === selectedId);
    if (!profile) return;
    const ok = await ask(`删除模型配置「${profile.name}」？`, {
      title: "确认删除",
      kind: "warning",
      okLabel: "删除",
      cancelLabel: "取消",
    });
    if (!ok) return;
    try {
      const next = await api.removeModel(selectedId);
      setStore(next);
      setSelectedId(null);
      log("OK", `已删除：${profile.name}`);
      if (status.running && store.default_id === profile.id) {
        log("DIM", "删除的是默认模型，建议重启网关。");
      }
    } catch (e) {
      log("ERR", String(e));
    }
  };

  const onToggleAutostart = async () => {
    const enable = !info?.autostart;
    await withBusy(enable ? "启用登录自启" : "关闭登录自启", async () => {
      try {
        const msg = await api.toggleAutostart(enable);
        log("DIM", msg);
        const proj = await api.projectInfo();
        setInfo(proj);
        return {
          ok: true,
          message: msg,
          logs: [msg],
          status,
        };
      } catch (e) {
        return {
          ok: false,
          message: String(e),
          logs: [String(e)],
          status,
        };
      }
    });
  };

  const saveModel = async (input: ModelInput, editId?: string) => {
    try {
      const next = editId
        ? await api.editModel(editId, input)
        : await api.createModel(input);
      setStore(next);
      setModelDialog(null);
      log("OK", editId ? `已更新：${input.name}` : `已保存：${input.name}`);

      const touchedDefault =
        editId &&
        (editId === store.default_id ||
          next.default_id === editId);
      if (status.running && (touchedDefault || !editId && next.profiles.length === 1)) {
        const yes = await ask("模型配置已变更。是否立即重启网关以生效？", {
          title: "重启网关",
          kind: "info",
          okLabel: "立即重启",
          cancelLabel: "稍后",
        });
        if (yes) await withBusy("重启网关", () => api.restart());
      }
    } catch (e) {
      await message(String(e), { title: "保存失败", kind: "error" });
      throw e;
    }
  };

  const running = status.healthy || status.running;

  return (
    <div className="app">
      {!booted && (
        <div className={`boot ${bootExit ? "exit" : ""}`}>
          <div className="boot-inner">
            <img className="boot-logo" src={logo} alt="logo" />
            <h1 className="boot-brand">Codex Chat Gateway</h1>
            <p className="boot-sub">private model bridge</p>
            <button type="button" className="boot-enter" onClick={enter}>
              点击进入
            </button>
            <p className="boot-hint">仅监听 127.0.0.1 · 密钥只保存在本机</p>
          </div>
        </div>
      )}

      <Particles running={running && status.is_our_gateway} />

      {booted && (
        <div className="shell">
          <header className="header">
            <img className="header-logo" src={logo} alt="" />
            <div>
              <div className="header-kicker">LOCAL MODEL BRIDGE / WINDOWS</div>
              <h1 className="header-title">Codex Chat Gateway</h1>
              <div className="header-meta">
                {info?.version ?? "…"} · 仅监听 127.0.0.1 · 密钥只保存在本机
              </div>
            </div>
            <button
              type="button"
              className="btn"
              onClick={() =>
                void openUrl(
                  "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows",
                )
              }
            >
              GitHub 仓库
            </button>
          </header>

          <div className="body">
            <section className="card panel">
              <p className="section-label">网关状态 GATEWAY</p>
              <div className="status-row">
                <span className={`status-dot ${running ? "on" : ""}`} />
                <span className={`status-text ${running ? "on" : ""}`}>
                  {running ? (status.is_our_gateway ? "运行中" : "端口占用") : "已停止"}
                </span>
              </div>
              <div className="endpoint-row">
                <span className="endpoint">{status.endpoint}</span>
                <button type="button" className="btn sm" onClick={onCopyEndpoint}>
                  复制
                </button>
              </div>
              <div className="detail">
                {running
                  ? [
                      status.pid != null ? `PID ${status.pid}` : null,
                      status.uptime ? `已运行 ${status.uptime}` : null,
                      status.model || null,
                    ]
                      .filter(Boolean)
                      .join(" · ") || status.message
                  : "网关未在运行 · 启动后此处显示 PID 与运行时长"}
              </div>
              <div className="model-line">
                默认模型：{status.default_model_name || store.profiles.find((p) => p.id === store.default_id)?.name || "未配置"}
              </div>

              <div className="btn-row cols-3">
                <button
                  type="button"
                  className="btn primary"
                  disabled={busy || (running && status.is_our_gateway)}
                  onClick={onStart}
                >
                  启动网关
                </button>
                <button
                  type="button"
                  className="btn"
                  disabled={busy || !running}
                  onClick={onStop}
                >
                  停止
                </button>
                <button
                  type="button"
                  className="btn"
                  disabled={busy || !running}
                  onClick={onRestart}
                >
                  重启
                </button>
              </div>
              <div className="btn-row cols-3">
                <button type="button" className="btn" disabled={busy} onClick={onCheck}>
                  检查接口
                </button>
                <button type="button" className="btn" onClick={() => void onOpenLogs()}>
                  打开日志
                </button>
                <button type="button" className="btn" onClick={onOpenUi}>
                  打开地址
                </button>
              </div>

              <div className="clients">
                <p className="section-label">客户端接入 CLIENTS</p>
                <div className="stack-gap">
                  <button
                    type="button"
                    className="btn ghost-wide"
                    disabled={busy}
                    onClick={() =>
                      void runClientScript(
                        "配置 Codex",
                        "configure-codex.ps1",
                      )
                    }
                  >
                    <span>配置 Codex</span>
                    <span className="sub">Responses API → 本地网关（先备份，保留 MCP）</span>
                  </button>
                  <button
                    type="button"
                    className="btn ghost-wide"
                    disabled={busy}
                    onClick={() =>
                      void runClientScript(
                        "配置 Claude Desktop",
                        "configure-claude-desktop.ps1",
                      )
                    }
                  >
                    <span>配置 Claude Desktop</span>
                    <span className="sub">Code 模式 → 本地网关（独立 3P Profile）</span>
                  </button>
                  <div className="btn-row cols-2">
                    <button
                      type="button"
                      className="btn"
                      disabled={busy}
                      onClick={() =>
                        void runClientScript(
                          "恢复 Codex 官方配置",
                          "restore-codex.ps1",
                          "将撤销网关相关的 Codex 配置并尽量恢复官方设置。继续？",
                        )
                      }
                    >
                      恢复 Codex
                    </button>
                    <button
                      type="button"
                      className="btn"
                      disabled={busy}
                      onClick={() =>
                        void runClientScript(
                          "恢复 Claude 官方配置",
                          "restore-claude-desktop.ps1",
                          "将移除本项目 3P Profile 并切回官方 1P 模式。继续？",
                        )
                      }
                    >
                      恢复 Claude
                    </button>
                  </div>
                  <button
                    type="button"
                    className="btn ghost-wide"
                    disabled={busy}
                    onClick={() => void onToggleAutostart()}
                  >
                    <span>
                      登录自启 · {info?.autostart ? "已开启" : "已关闭"}
                    </span>
                    <span className="sub">
                      {info?.autostart
                        ? "点击关闭登录自启"
                        : "点击开启：登录 Windows 后自动启动网关"}
                    </span>
                  </button>
                </div>
              </div>
            </section>

            <section className="card panel">
              <div className="models-head">
                <p className="section-label" style={{ margin: 0 }}>
                  模型配置 MODELS
                </p>
                <button
                  type="button"
                  className="btn primary sm"
                  disabled={busy}
                  onClick={() => setModelDialog({ mode: "add" })}
                >
                  ＋ 添加模型
                </button>
              </div>
              <p className="hint">
                密钥仅保存在当前用户的 .gateway/models.json，不会写入任何客户端配置。
              </p>
              <div className="table-wrap">
                {store.profiles.length === 0 ? (
                  <div className="empty">还没有模型配置。点击右上角添加。</div>
                ) : (
                  <table className="models">
                    <thead>
                      <tr>
                        <th>配置名称</th>
                        <th>上游模型</th>
                        <th>适配器</th>
                        <th>API 地址</th>
                      </tr>
                    </thead>
                    <tbody>
                      {store.profiles.map((p) => {
                        const isDefault = p.id === store.default_id;
                        return (
                          <tr
                            key={p.id}
                            className={[
                              selectedId === p.id ? "selected" : "",
                              isDefault ? "default" : "",
                            ].join(" ")}
                            onClick={() => setSelectedId(p.id)}
                            onDoubleClick={() =>
                              setModelDialog({ mode: "edit", profile: p })
                            }
                          >
                            <td>
                              {isDefault ? "● " : ""}
                              {p.name}
                            </td>
                            <td>{p.model_id}</td>
                            <td>{p.litellm_model}</td>
                            <td title={p.base_url}>{p.base_url}</td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                )}
              </div>
              <div className="model-actions">
                <button type="button" className="btn" disabled={busy} onClick={() => void onSetDefault()}>
                  设为默认
                </button>
                <button
                  type="button"
                  className="btn"
                  disabled={busy || !selectedId}
                  onClick={() => {
                    const p = store.profiles.find((x) => x.id === selectedId);
                    if (p) setModelDialog({ mode: "edit", profile: p });
                    else log("DIM", "请先选择一个模型。");
                  }}
                >
                  编辑
                </button>
                <button type="button" className="btn danger" disabled={busy} onClick={() => void onDelete()}>
                  删除
                </button>
                <button type="button" className="btn" disabled={busy} onClick={() => void refresh()}>
                  刷新
                </button>
              </div>
            </section>
          </div>

          <section className="card console">
            <div className="console-bar">
              <span className="section-label" style={{ margin: 0 }}>
                输出 OUTPUT
              </span>
              <div style={{ display: "flex", gap: 6 }}>
                <button
                  type="button"
                  className="btn sm"
                  onClick={() => {
                    const text = logs.map((l) => `${l.level}  ${l.message}`).join("\n");
                    if (safeClipboard(text)) log("OK", "已复制输出");
                  }}
                >
                  复制
                </button>
                <button type="button" className="btn sm" onClick={() => setLogs([])}>
                  清空
                </button>
              </div>
            </div>
            <div className="console-body" ref={consoleRef}>
              {logs.length === 0 ? (
                <div className="log-line">
                  <span className="log-level DIM">DIM</span>
                  <span className="log-msg">等待操作…</span>
                </div>
              ) : (
                logs.map((l) => (
                  <div className="log-line" key={l.id}>
                    <span className={`log-level ${l.level}`}>{l.level}</span>
                    <span className="log-msg">{l.message}</span>
                  </div>
                ))
              )}
            </div>
          </section>
        </div>
      )}

      {modelDialog && (
        <ModelDialog
          mode={modelDialog.mode}
          profile={modelDialog.profile}
          onClose={() => setModelDialog(null)}
          onSave={saveModel}
        />
      )}
    </div>
  );
}

function ModelDialog({
  mode,
  profile,
  onClose,
  onSave,
}: {
  mode: "add" | "edit";
  profile?: ModelProfile;
  onClose: () => void;
  onSave: (input: ModelInput, editId?: string) => Promise<void>;
}) {
  const [name, setName] = useState(profile?.name ?? "");
  const [url, setUrl] = useState(profile?.base_url ?? "");
  const [key, setKey] = useState(profile?.api_key ?? "");
  const [showKey, setShowKey] = useState(false);
  const [modelId, setModelId] = useState(profile?.model_id ?? "");
  const [status, setStatus] = useState<{ text: string; kind: "" | "err" | "ok" }>({
    text: "",
    kind: "",
  });
  const [fetching, setFetching] = useState(false);
  const [picker, setPicker] = useState<string[] | null>(null);
  const [filter, setFilter] = useState("");
  const [saving, setSaving] = useState(false);

  const filtered = useMemo(() => {
    if (!picker) return [];
    const n = filter.trim().toLowerCase();
    if (!n) return picker;
    return picker.filter((id) => id.toLowerCase().includes(n));
  }, [picker, filter]);

  const fetchList = async () => {
    const base = url.trim().replace(/\/+$/, "");
    if (!/^https?:\/\//i.test(base)) {
      setStatus({ text: "请先填写有效的 HTTP(S) API 地址。", kind: "err" });
      return;
    }
    if (!key.trim()) {
      setStatus({ text: "请先填写 API Key。", kind: "err" });
      return;
    }
    setFetching(true);
    setStatus({ text: "正在获取模型列表…", kind: "" });
    try {
      const ids = await api.fetchModels(base, key.trim());
      setPicker(ids);
      setStatus({ text: `已获取 ${ids.length} 个模型，请选择。`, kind: "ok" });
    } catch (e) {
      setStatus({ text: `${String(e)} 可改用手动输入模型 ID。`, kind: "err" });
    } finally {
      setFetching(false);
    }
  };

  const submit = async () => {
    setSaving(true);
    try {
      await onSave(
        {
          name: name.trim(),
          base_url: url.trim().replace(/\/+$/, ""),
          api_key: key,
          model_id: modelId.trim(),
        },
        mode === "edit" ? profile?.id : undefined,
      );
    } catch {
      // parent shows error
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal card" onClick={(e) => e.stopPropagation()}>
        <h2>{mode === "add" ? "添加模型" : "编辑模型"}</h2>
        {picker ? (
          <>
            <div className="field">
              <label>搜索模型</label>
              <input
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                placeholder="输入关键字过滤"
                autoFocus
              />
            </div>
            <div className="picker-list">
              {filtered.map((id) => (
                <button
                  key={id}
                  type="button"
                  className={`picker-item ${modelId === id ? "active" : ""}`}
                  onClick={() => {
                    setModelId(id);
                    if (!name.trim()) setName(id);
                    setPicker(null);
                    setStatus({ text: `已选择 ${id}`, kind: "ok" });
                  }}
                >
                  {id}
                </button>
              ))}
              {filtered.length === 0 && <div className="empty">无匹配模型</div>}
            </div>
            <div className="modal-actions">
              <button type="button" className="btn" onClick={() => setPicker(null)}>
                返回
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="field">
              <label>配置名称</label>
              <input value={name} onChange={(e) => setName(e.target.value)} />
            </div>
            <div className="field">
              <label>API Base URL（通常以 /v1 结尾）</label>
              <input
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://api.example.com/v1"
              />
            </div>
            <div className="field">
              <label>API Key</label>
              <div className="field-row">
                <input
                  type={showKey ? "text" : "password"}
                  value={key}
                  onChange={(e) => setKey(e.target.value)}
                />
                <button type="button" className="btn sm" onClick={() => setShowKey((v) => !v)}>
                  {showKey ? "隐藏" : "显示"}
                </button>
              </div>
            </div>
            <div className="field">
              <label>模型 ID</label>
              <div className="field-row">
                <input
                  value={modelId}
                  onChange={(e) => setModelId(e.target.value)}
                  placeholder="deepseek-chat"
                />
                <button
                  type="button"
                  className="btn sm"
                  disabled={fetching}
                  onClick={() => void fetchList()}
                >
                  {fetching ? "获取中…" : "在线获取"}
                </button>
              </div>
            </div>
            <div className={`field-status ${status.kind}`}>{status.text}</div>
            <div className="modal-actions">
              <button type="button" className="btn" onClick={onClose} disabled={saving}>
                取消
              </button>
              <button
                type="button"
                className="btn primary"
                disabled={saving}
                onClick={() => void submit()}
              >
                保存配置
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
