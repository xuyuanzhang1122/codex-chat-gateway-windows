import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { ask, message } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { TitleBar } from "./TitleBar";
import { IconActivity, IconClients, IconGateway, IconModels } from "./icons";
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

type Page = "gateway" | "models" | "clients" | "activity";

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
  message: "检测中…",
  routes: [],
};

function copyText(text: string): boolean {
  try {
    void navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const el = document.createElement("textarea");
      el.value = text;
      document.body.appendChild(el);
      el.select();
      const ok = document.execCommand("copy");
      document.body.removeChild(el);
      return ok;
    } catch {
      return false;
    }
  }
}

export default function App() {
  const [splash, setSplash] = useState(true);
  const [splashExit, setSplashExit] = useState(false);
  const [page, setPage] = useState<Page>("gateway");
  const [status, setStatus] = useState<GatewayStatus>(emptyStatus);
  const [store, setStore] = useState<ModelStore>({ version: 1, default_id: "", profiles: [] });
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const logSeq = useRef(0);
  const logRef = useRef<HTMLDivElement | null>(null);
  const [dialog, setDialog] = useState<null | { mode: "add" | "edit"; profile?: ModelProfile }>(
    null,
  );
  const ready = !splash;

  const pushLog = useCallback((level: LogLevel, msg: string) => {
    if (!msg) return;
    setLogs((prev) => {
      const next = [...prev, { id: ++logSeq.current, level, message: msg }];
      return next.length > 300 ? next.slice(-300) : next;
    });
  }, []);

  useEffect(() => {
    if (page === "activity") {
      logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
    }
  }, [logs, page]);

  const loadAll = useCallback(async () => {
    const [st, models, proj] = await Promise.all([
      api.status(),
      api.listModels(),
      api.projectInfo(),
    ]);
    setStatus(st);
    setStore(models);
    setInfo(proj);
  }, []);

  // Light status poll only (no full store refresh) — major perf win
  useEffect(() => {
    if (!ready) return;
    let cancelled = false;
    let timer = 0;

    const tick = async () => {
      try {
        const st = await api.status();
        if (!cancelled) setStatus(st);
      } catch {
        /* ignore transient */
      }
      if (!cancelled) timer = window.setTimeout(tick, 5000);
    };

    void (async () => {
      try {
        await loadAll();
        pushLog("INFO", "Studio 控制台已就绪");
        const proj = await api.projectInfo();
        pushLog("DIM", proj.root);
      } catch (e) {
        pushLog("ERR", `初始化失败: ${String(e)}`);
      }
      if (!cancelled) timer = window.setTimeout(tick, 5000);
    })();

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [ready, loadAll, pushLog]);

  const enter = () => {
    setSplashExit(true);
    window.setTimeout(() => setSplash(false), 420);
  };

  const applyResult = (label: string, result: ActionResult) => {
    for (const line of result.logs) pushLog(result.ok ? "DIM" : "ERR", line);
    pushLog(
      result.ok ? "OK" : "ERR",
      result.ok ? `${label} 完成` : `${label} 失败 · ${result.message}`,
    );
    setStatus(result.status);
  };

  const run = async (label: string, fn: () => Promise<ActionResult | void>) => {
    if (busy) return;
    setBusy(true);
    pushLog("INFO", `▶ ${label}`);
    try {
      const result = await fn();
      if (result) applyResult(label, result);
      const models = await api.listModels();
      setStore(models);
      const st = await api.status();
      setStatus(st);
      const proj = await api.projectInfo();
      setInfo(proj);
    } catch (e) {
      pushLog("ERR", `${label} 异常 · ${String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const hasDefault = store.profiles.some((p) => p.id === store.default_id);
  const live = status.healthy || status.running;

  const pageMeta = useMemo(() => {
    switch (page) {
      case "gateway":
        return {
          kicker: "Runtime",
          title: "网关运行时",
          sub: "本机 LiteLLM 桥 · 仅 127.0.0.1",
        };
      case "models":
        return {
          kicker: "Profiles",
          title: "上游模型",
          sub: "密钥只保存在本机 .gateway/models.json",
        };
      case "clients":
        return {
          kicker: "Clients",
          title: "客户端接入",
          sub: "Codex / Claude Desktop · 可安全恢复",
        };
      case "activity":
        return {
          kicker: "Activity",
          title: "运行日志",
          sub: "桌面操作与脚本输出",
        };
    }
  }, [page]);

  const onStart = () => {
    if (!hasDefault) {
      pushLog("DIM", "请先添加默认模型");
      setPage("models");
      setDialog({ mode: "add" });
      return;
    }
    void run("启动网关", () => api.start());
  };

  const saveModel = async (input: ModelInput, editId?: string) => {
    const next = editId
      ? await api.editModel(editId, input)
      : await api.createModel(input);
    setStore(next);
    setDialog(null);
    pushLog("OK", editId ? `已更新 ${input.name}` : `已保存 ${input.name}`);

    const touchedDefault =
      (editId && editId === store.default_id) || (!editId && next.profiles.length === 1);
    if (live && touchedDefault) {
      const yes = await ask("配置已变更，是否立即重启网关？", {
        title: "重启网关",
        kind: "info",
        okLabel: "重启",
        cancelLabel: "稍后",
      });
      if (yes) await run("重启网关", () => api.restart());
    }
  };

  return (
    <>
      {splash && (
        <div className={`splash ${splashExit ? "exit" : ""}`}>
          <div className="splash-inner">
            <img className="splash-logo" src="/gateway-logo.png" alt="" />
            <h1>Codex Chat Gateway</h1>
            <p>studio console</p>
            <button type="button" className="splash-cta" onClick={enter}>
              进入
            </button>
            <p className="splash-note">本地模型桥 · 密钥不出本机</p>
          </div>
        </div>
      )}

      <div className="app-frame">
        <TitleBar />
        <div className="shell">
          <aside className="rail">
            <nav className="rail-nav">
              <RailButton
                active={page === "gateway"}
                title="网关"
                onClick={() => setPage("gateway")}
              >
                <IconGateway />
              </RailButton>
              <RailButton
                active={page === "models"}
                title="模型"
                onClick={() => setPage("models")}
              >
                <IconModels />
              </RailButton>
              <RailButton
                active={page === "clients"}
                title="客户端"
                onClick={() => setPage("clients")}
              >
                <IconClients />
              </RailButton>
              <RailButton
                active={page === "activity"}
                title="日志"
                onClick={() => setPage("activity")}
              >
                <IconActivity />
              </RailButton>
            </nav>
            <div className="rail-foot" title={status.message}>
              <div className={`rail-dot ${live && status.is_our_gateway ? "on" : ""}`} />
            </div>
          </aside>

          <main className="stage">
            <div className="stage-head">
              <div>
                <p className="stage-kicker">{pageMeta.kicker}</p>
                <h2 className="stage-title">{pageMeta.title}</h2>
                <p className="stage-sub">{pageMeta.sub}</p>
              </div>
              <div className="chip">
                <span>{info?.version ?? "…"}</span>
              </div>
            </div>

            <div className="stage-body" key={page}>
              {page === "gateway" && (
                <GatewayPage
                  status={status}
                  store={store}
                  busy={busy}
                  live={live}
                  onStart={onStart}
                  onStop={() => void run("停止网关", () => api.stop())}
                  onRestart={() => {
                    if (!hasDefault) {
                      pushLog("DIM", "请先配置默认模型");
                      setPage("models");
                      return;
                    }
                    void run("重启网关", () => api.restart());
                  }}
                  onCheck={() => void run("接口检查", () => api.check())}
                  onCopy={() => {
                    if (copyText(status.endpoint)) pushLog("OK", "已复制接口地址");
                    else pushLog("ERR", "剪贴板不可用");
                  }}
                  onLogs={async () => {
                    try {
                      const dir = await api.logsDir();
                      await openPath(dir);
                      pushLog("OK", `日志目录 ${dir}`);
                    } catch (e) {
                      pushLog("ERR", String(e));
                    }
                  }}
                  onUi={() => void openUrl("http://127.0.0.1:4000/ui")}
                />
              )}

              {page === "models" && (
                <ModelsPage
                  store={store}
                  selectedId={selectedId}
                  busy={busy}
                  onSelect={setSelectedId}
                  onAdd={() => setDialog({ mode: "add" })}
                  onEdit={() => {
                    const p = store.profiles.find((x) => x.id === selectedId);
                    if (!p) {
                      pushLog("DIM", "请先选择模型卡片");
                      return;
                    }
                    setDialog({ mode: "edit", profile: p });
                  }}
                  onDefault={async () => {
                    if (!selectedId) {
                      pushLog("DIM", "请先选择模型卡片");
                      return;
                    }
                    try {
                      const next = await api.makeDefault(selectedId);
                      setStore(next);
                      pushLog("OK", "已设为默认模型");
                      if (live) {
                        const yes = await ask("是否立即重启网关使默认模型生效？", {
                          title: "重启网关",
                          kind: "info",
                          okLabel: "重启",
                          cancelLabel: "稍后",
                        });
                        if (yes) await run("重启网关", () => api.restart());
                      }
                    } catch (e) {
                      pushLog("ERR", String(e));
                    }
                  }}
                  onDelete={async () => {
                    if (!selectedId) {
                      pushLog("DIM", "请先选择模型卡片");
                      return;
                    }
                    const p = store.profiles.find((x) => x.id === selectedId);
                    if (!p) return;
                    const ok = await ask(`删除「${p.name}」？`, {
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
                      pushLog("OK", `已删除 ${p.name}`);
                    } catch (e) {
                      pushLog("ERR", String(e));
                    }
                  }}
                  onRefresh={() => void loadAll().then(() => pushLog("OK", "已刷新模型列表"))}
                />
              )}

              {page === "clients" && (
                <ClientsPage
                  busy={busy}
                  autostart={!!info?.autostart}
                  onScript={(label, script, confirmText) => {
                    void (async () => {
                      if (confirmText) {
                        const ok = await ask(confirmText, {
                          title: label,
                          kind: "warning",
                          okLabel: "继续",
                          cancelLabel: "取消",
                        });
                        if (!ok) {
                          pushLog("DIM", `已取消 · ${label}`);
                          return;
                        }
                      }
                      await run(label, () => api.runScript(script));
                    })();
                  }}
                  onAutostart={() => {
                    const enable = !info?.autostart;
                    void run(enable ? "启用登录自启" : "关闭登录自启", async () => {
                      try {
                        const msg = await api.toggleAutostart(enable);
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
                  }}
                />
              )}

              {page === "activity" && (
                <div className="surface log-panel">
                  <div className="log-head">
                    <strong>输出流</strong>
                    <div className="btn-group">
                      <button
                        type="button"
                        className="btn sm"
                        onClick={() => {
                          const text = logs.map((l) => `${l.level}  ${l.message}`).join("\n");
                          if (copyText(text)) pushLog("OK", "已复制日志");
                        }}
                      >
                        复制
                      </button>
                      <button type="button" className="btn sm" onClick={() => setLogs([])}>
                        清空
                      </button>
                    </div>
                  </div>
                  <div className="log-body" ref={logRef}>
                    {logs.length === 0 ? (
                      <div className="log-line">
                        <span className="log-lv DIM">DIM</span>
                        <span className="log-msg">暂无输出</span>
                      </div>
                    ) : (
                      logs.map((l) => (
                        <div className="log-line" key={l.id}>
                          <span className={`log-lv ${l.level}`}>{l.level}</span>
                          <span className="log-msg">{l.message}</span>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              )}
            </div>
          </main>
        </div>
      </div>

      {dialog && (
        <ModelDialog
          mode={dialog.mode}
          profile={dialog.profile}
          onClose={() => setDialog(null)}
          onSave={async (input, editId) => {
            try {
              await saveModel(input, editId);
            } catch (e) {
              await message(String(e), { title: "保存失败", kind: "error" });
              throw e;
            }
          }}
        />
      )}
    </>
  );
}

function RailButton({
  active,
  title,
  onClick,
  children,
}: {
  active: boolean;
  title: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      className={`rail-btn ${active ? "active" : ""}`}
      title={title}
      aria-label={title}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function GatewayPage({
  status,
  store,
  busy,
  live,
  onStart,
  onStop,
  onRestart,
  onCheck,
  onCopy,
  onLogs,
  onUi,
}: {
  status: GatewayStatus;
  store: ModelStore;
  busy: boolean;
  live: boolean;
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onCheck: () => void;
  onCopy: () => void;
  onLogs: () => void;
  onUi: () => void;
}) {
  const stateLabel = !live
    ? "已停止"
    : status.is_our_gateway
      ? "运行中"
      : "端口占用";
  const stateClass = !live ? "off" : status.is_our_gateway ? "ok" : "warn";
  const defaultName =
    status.default_model_name ||
    store.profiles.find((p) => p.id === store.default_id)?.name ||
    "未配置";

  return (
    <div className="stack">
      <div className="grid-metrics">
        <div className="surface metric">
          <div className="metric-label">状态</div>
          <div className={`metric-value ${stateClass}`}>{stateLabel}</div>
          <div className="metric-foot">{status.message}</div>
        </div>
        <div className="surface metric">
          <div className="metric-label">进程</div>
          <div className="metric-value">{status.pid ?? "—"}</div>
          <div className="metric-foot">
            {status.uptime ? `已运行 ${status.uptime}` : "启动后显示运行时长"}
          </div>
        </div>
        <div className="surface metric">
          <div className="metric-label">默认模型</div>
          <div className="metric-value" style={{ fontSize: 18 }}>
            {defaultName}
          </div>
          <div className="metric-foot">{status.model || "—"}</div>
        </div>
      </div>

      <div className="surface panel">
        <div className="panel-title">控制</div>
        <div className="row" style={{ marginBottom: 12 }}>
          <div className="chip">
            {status.endpoint}
            <button type="button" className="btn sm" onClick={onCopy}>
              复制
            </button>
          </div>
        </div>
        <div className="btn-group">
          <button
            type="button"
            className="btn teal"
            disabled={busy || (live && status.is_our_gateway)}
            onClick={onStart}
          >
            启动
          </button>
          <button type="button" className="btn" disabled={busy || !live} onClick={onStop}>
            停止
          </button>
          <button type="button" className="btn" disabled={busy || !live} onClick={onRestart}>
            重启
          </button>
          <button type="button" className="btn" disabled={busy} onClick={onCheck}>
            健康检查
          </button>
          <button type="button" className="btn ghost" onClick={onLogs}>
            打开日志目录
          </button>
          <button type="button" className="btn ghost" onClick={onUi}>
            LiteLLM UI
          </button>
        </div>
      </div>
    </div>
  );
}

function ModelsPage({
  store,
  selectedId,
  busy,
  onSelect,
  onAdd,
  onEdit,
  onDefault,
  onDelete,
  onRefresh,
}: {
  store: ModelStore;
  selectedId: string | null;
  busy: boolean;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onEdit: () => void;
  onDefault: () => void;
  onDelete: () => void;
  onRefresh: () => void;
}) {
  return (
    <div>
      <div className="models-toolbar">
        <span style={{ color: "var(--text-dim)", fontSize: 12.5 }}>
          {store.profiles.length} 个配置
        </span>
        <div className="btn-group">
          <button type="button" className="btn primary" disabled={busy} onClick={onAdd}>
            添加模型
          </button>
          <button type="button" className="btn" disabled={busy} onClick={onRefresh}>
            刷新
          </button>
        </div>
      </div>

      {store.profiles.length === 0 ? (
        <div className="empty-state">
          还没有上游模型。添加后即可启动本地网关。
          <div style={{ marginTop: 14 }}>
            <button type="button" className="btn primary" onClick={onAdd}>
              立即添加
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className="models-grid">
            {store.profiles.map((p) => {
              const isDefault = p.id === store.default_id;
              return (
                <button
                  type="button"
                  key={p.id}
                  className={`model-card ${selectedId === p.id ? "selected" : ""} ${isDefault ? "default" : ""}`}
                  onClick={() => onSelect(p.id)}
                  onDoubleClick={onEdit}
                >
                  <div className="model-name">{p.name}</div>
                  <div className="model-meta">
                    <div>{p.model_id}</div>
                    <div>{p.litellm_model}</div>
                    <div>{p.base_url}</div>
                  </div>
                </button>
              );
            })}
          </div>
          <div className="model-actions">
            <button type="button" className="btn" disabled={busy} onClick={onDefault}>
              设为默认
            </button>
            <button type="button" className="btn" disabled={busy} onClick={onEdit}>
              编辑
            </button>
            <button type="button" className="btn danger" disabled={busy} onClick={onDelete}>
              删除
            </button>
          </div>
        </>
      )}
    </div>
  );
}

function ClientsPage({
  busy,
  autostart,
  onScript,
  onAutostart,
}: {
  busy: boolean;
  autostart: boolean;
  onScript: (label: string, script: string, confirm?: string) => void;
  onAutostart: () => void;
}) {
  return (
    <div className="client-grid">
      <div className="surface client-card">
        <h3>Codex</h3>
        <p>写入本地 Responses 提供方，自动备份并保留 MCP / 插件配置。</p>
        <div className="btn-group">
          <button
            type="button"
            className="btn primary"
            disabled={busy}
            onClick={() => onScript("配置 Codex", "configure-codex.ps1")}
          >
            配置
          </button>
          <button
            type="button"
            className="btn danger"
            disabled={busy}
            onClick={() =>
              onScript(
                "恢复 Codex",
                "restore-codex.ps1",
                "撤销网关相关 Codex 配置并尽量恢复官方设置？",
              )
            }
          >
            恢复官方
          </button>
        </div>
      </div>

      <div className="surface client-card">
        <h3>Claude Desktop</h3>
        <p>仅配置 Code 模式 3P Profile，不改普通聊天、MCP 或应用本体。</p>
        <div className="btn-group">
          <button
            type="button"
            className="btn primary"
            disabled={busy}
            onClick={() => onScript("配置 Claude Desktop", "configure-claude-desktop.ps1")}
          >
            配置
          </button>
          <button
            type="button"
            className="btn danger"
            disabled={busy}
            onClick={() =>
              onScript(
                "恢复 Claude",
                "restore-claude-desktop.ps1",
                "移除本项目 Profile 并切回官方 1P 模式？",
              )
            }
          >
            恢复官方
          </button>
        </div>
      </div>

      <div className="surface client-card" style={{ gridColumn: "1 / -1" }}>
        <h3>系统</h3>
        <p>登录 Windows 后自动启动网关后台进程（不启动本控制台）。</p>
        <button type="button" className="btn block" disabled={busy} onClick={onAutostart}>
          <span>登录自启 · {autostart ? "已开启" : "已关闭"}</span>
          <span className="sub">{autostart ? "点击关闭" : "点击开启"}</span>
        </button>
      </div>
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
  const [msg, setMsg] = useState<{ text: string; kind: "" | "err" | "ok" }>({
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
    return n ? picker.filter((id) => id.toLowerCase().includes(n)) : picker;
  }, [picker, filter]);

  const fetchList = async () => {
    const base = url.trim().replace(/\/+$/, "");
    if (!/^https?:\/\//i.test(base)) {
      setMsg({ text: "请填写有效的 HTTP(S) 地址", kind: "err" });
      return;
    }
    if (!key.trim()) {
      setMsg({ text: "请填写 API Key", kind: "err" });
      return;
    }
    setFetching(true);
    setMsg({ text: "正在拉取模型列表…", kind: "" });
    try {
      const ids = await api.fetchModels(base, key.trim());
      setPicker(ids);
      setMsg({ text: `共 ${ids.length} 个模型`, kind: "ok" });
    } catch (e) {
      setMsg({ text: `${String(e)} · 可手动填写模型 ID`, kind: "err" });
    } finally {
      setFetching(false);
    }
  };

  return (
    <div className="modal-root" onClick={onClose}>
      <div className="surface modal" onClick={(e) => e.stopPropagation()}>
        <h2>{mode === "add" ? "添加上游模型" : "编辑模型"}</h2>
        <p className="hint">按 API URL → Key → 模型 的顺序配置</p>

        {picker ? (
          <>
            <div className="field">
              <label>过滤</label>
              <input
                value={filter}
                onChange={(e) => setFilter(e.target.value)}
                placeholder="搜索模型 ID"
                autoFocus
              />
            </div>
            <div className="picker">
              {filtered.map((id) => (
                <button
                  key={id}
                  type="button"
                  className={modelId === id ? "active" : ""}
                  onClick={() => {
                    setModelId(id);
                    if (!name.trim()) setName(id);
                    setPicker(null);
                    setMsg({ text: `已选择 ${id}`, kind: "ok" });
                  }}
                >
                  {id}
                </button>
              ))}
              {filtered.length === 0 && (
                <div style={{ padding: 16, color: "var(--text-faint)" }}>无匹配项</div>
              )}
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
              <label>名称</label>
              <input value={name} onChange={(e) => setName(e.target.value)} />
            </div>
            <div className="field">
              <label>API Base URL</label>
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
                  {fetching ? "…" : "在线获取"}
                </button>
              </div>
            </div>
            <div className={`field-msg ${msg.kind}`}>{msg.text}</div>
            <div className="modal-actions">
              <button type="button" className="btn" disabled={saving} onClick={onClose}>
                取消
              </button>
              <button
                type="button"
                className="btn primary"
                disabled={saving}
                onClick={() => {
                  void (async () => {
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
                    } finally {
                      setSaving(false);
                    }
                  })();
                }}
              >
                保存
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
