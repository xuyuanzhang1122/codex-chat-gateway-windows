import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ActionIcon,
  Alert,
  Block,
  Button,
  Empty,
  Flexbox,
  Form,
  FormItem,
  Icon,
  Input,
  InputPassword,
  Modal,
  SearchBar,
  SideNav,
  Tag,
  Text,
  Tooltip,
} from "@lobehub/ui";
import {
  Activity,
  AlertTriangle,
  Bot,
  CheckCircle2,
  Copy,
  ExternalLink,
  FolderOpen,
  GitBranch,
  Layers3,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  RotateCcw,
  Server,
  Settings2,
  Square,
  Star,
  Trash2,
  Users,
  Download,
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check as checkUpdate } from "@tauri-apps/plugin-updater";
import { InputNumber, Switch } from "antd";
import { api } from "./api";
import { TitleBar } from "./TitleBar";
import { RoutingMapView } from "./RoutingMap";
import { installKnownUpdate, type UpdateProgress } from "./updater";
import type {
  GatewayStatus,
  LogLevel,
  LogLine,
  ModelInput,
  ModelProfile,
  ModelStore,
  ParsedApiText,
  ProjectInfo,
  RoutingTrafficStore,
} from "./types";

type Page = "gateway" | "routing" | "models" | "clients" | "activity";

type ActionKey =
  | "start"
  | "stop"
  | "restart"
  | "reload"
  | "check"
  | "codex-cfg"
  | "codex-restore"
  | "claude-cfg"
  | "claude-restore"
  | "autostart"
  | "logs"
  | "default"
  | "routing"
  | "delete"
  | "import"
  | "update";

type Feedback = {
  key: ActionKey;
  state: "loading" | "ok" | "err";
  message: string;
};

type ConfirmRequest = {
  title: string;
  content: string;
  okText?: string;
  cancelText?: string;
  danger?: boolean;
  resolve: (ok: boolean) => void;
};

type NoticeRequest = {
  title: string;
  content: string;
  type?: "error" | "info" | "success";
};

const emptyStatus: GatewayStatus = {
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
  message: "检测中…",
  routes: [],
  busy: false,
  startup_progress: null,
  startup_stage: null,
};

const GITHUB = "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows";
const LOBE_UI = "https://ui.lobehub.com";

async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch {
    /* fall through */
  }
  try {
    const el = document.createElement("textarea");
    el.value = text;
    el.style.position = "fixed";
    el.style.left = "-9999px";
    document.body.appendChild(el);
    el.select();
    const ok = document.execCommand("copy");
    document.body.removeChild(el);
    return ok;
  } catch {
    return false;
  }
}

function App() {
  const [page, setPage] = useState<Page>("gateway");
  const [status, setStatus] = useState<GatewayStatus>(emptyStatus);
  const [store, setStore] = useState<ModelStore>({
    version: 3,
    default_id: "",
    profiles: [],
    routing: { enabled: false, affinity_ttl_seconds: 3600, model_rules: [] },
  });
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [traffic, setTraffic] = useState<RoutingTrafficStore>({ version: 1, routes: [] });
  const [trafficError, setTrafficError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const logId = useRef(0);
  const [dialog, setDialog] = useState<null | { mode: "add" | "edit"; profile?: ModelProfile }>(
    null,
  );
  const [feedback, setFeedback] = useState<Feedback | null>(null);
  const [confirmReq, setConfirmReq] = useState<ConfirmRequest | null>(null);
  const [notice, setNotice] = useState<NoticeRequest | null>(null);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress | null>(null);
  const pendingKey = useRef<ActionKey | null>(null);
  const feedbackTimer = useRef<number | null>(null);
  const autoUpdateChecked = useRef(false);
  const confirm = useCallback(
    (opts: Omit<ConfirmRequest, "resolve">) =>
      new Promise<boolean>((resolve) => {
        setConfirmReq({ ...opts, resolve });
      }),
    [],
  );

  const pushLog = useCallback((level: LogLevel, msg: string) => {
    if (!msg) return;
    setLogs((prev) => {
      const next = [...prev, { id: ++logId.current, level, message: msg }];
      return next.length > 250 ? next.slice(-250) : next;
    });
  }, []);

  const showFeedback = useCallback((fb: Feedback, autoClearMs = 4500) => {
    setFeedback(fb);
    if (feedbackTimer.current) window.clearTimeout(feedbackTimer.current);
    if (fb.state !== "loading" && autoClearMs > 0) {
      feedbackTimer.current = window.setTimeout(() => {
        setFeedback((cur) => (cur && cur.key === fb.key && cur.state !== "loading" ? null : cur));
      }, autoClearMs);
    }
  }, []);

  const beginAction = useCallback(
    (key: ActionKey, loadingMsg: string) => {
      pendingKey.current = key;
      showFeedback({ key, state: "loading", message: loadingMsg }, 0);
    },
    [showFeedback],
  );

  // Auto-select default / sole profile so actions work without an extra click
  useEffect(() => {
    if (store.profiles.length === 0) {
      setSelectedId(null);
      return;
    }
    setSelectedId((cur) => {
      if (cur && store.profiles.some((p) => p.id === cur)) return cur;
      return store.default_id || store.profiles[0].id;
    });
  }, [store]);

  // Event-driven backend (no heavy polling loop)
  useEffect(() => {
    let unsubs: Array<() => void> = [];

    void (async () => {
      try {
        const [st, models, proj] = await Promise.all([
          api.status(),
          api.listModels(),
          api.projectInfo(),
        ]);
        setStatus(st);
        setStore(models);
        setInfo(proj);
        pushLog("INFO", "Studio 已就绪");
        pushLog("DIM", proj.root);
      } catch (e) {
        pushLog("ERR", `初始化失败: ${String(e)}`);
      }

      const u1 = await listen<GatewayStatus>("gateway://status", (e) => {
        const st = e.payload;
        setStatus(st);
        // Unstick loading UI if backend finished busy but action event was missed.
        const key = pendingKey.current;
        if (key && !st.busy && (key === "start" || key === "stop" || key === "restart")) {
          if (key === "stop" && !st.running && !st.healthy) {
            showFeedback({ key, state: "ok", message: st.message || "网关已停止" });
            pendingKey.current = null;
          } else if (key === "start" && st.running && st.healthy) {
            showFeedback({ key, state: "ok", message: st.message || "网关已启动" });
            pendingKey.current = null;
          } else if (key === "restart" && st.running && st.healthy && st.phase === "running") {
            showFeedback({ key, state: "ok", message: st.message || "网关已重启" });
            pendingKey.current = null;
          } else if (
            (key === "stop" || key === "start" || key === "restart") &&
            st.phase === "error" &&
            !st.busy
          ) {
            showFeedback({
              key,
              state: "err",
              message: st.message || "操作失败",
            });
            pendingKey.current = null;
          }
        }
      });
      const u2 = await listen<{ level: string; message: string }>("gateway://log", (e) => {
        const lv = (e.payload.level || "DIM").toUpperCase() as LogLevel;
        pushLog(
          lv === "OK" || lv === "ERR" || lv === "INFO" || lv === "DIM" ? lv : "DIM",
          e.payload.message,
        );
      });
      const u3 = await listen<{ ok: boolean; message: string }>("gateway://action", (e) => {
        const key = pendingKey.current;
        if (key) {
          const raw = e.payload.message || "";
          const mapped =
            raw === "still running"
              ? "停止失败：端口仍有响应"
              : raw === "Gateway stopped"
                ? "网关已停止"
                : raw === "Gateway started"
                  ? "网关已启动"
                  : raw === "Gateway already running"
                    ? "网关已在运行"
                    : raw || (e.payload.ok ? "完成" : "失败");
          showFeedback({
            key,
            state: e.payload.ok ? "ok" : "err",
            message: mapped,
          });
          pendingKey.current = null;
        }
        // Always refresh status after lifecycle actions so buttons re-enable correctly.
        void api.status().then(setStatus).catch(() => undefined);
        if (e.payload.ok) {
          void api.listModels().then(setStore).catch(() => undefined);
          void api.projectInfo().then(setInfo).catch(() => undefined);
        }
      });
      unsubs = [u1, u2, u3];

      // Quiet background check — never auto-download; only notify in logs.
      if (!autoUpdateChecked.current) {
        autoUpdateChecked.current = true;
        void checkUpdate()
          .then((u) => {
            if (u) {
              pushLog(
                "INFO",
                `发现新版本 ${u.version}（当前 ${u.currentVersion}）· 可在「客户端」页检查更新`,
              );
            } else {
              pushLog("DIM", "已检查更新：当前为最新控制台版本");
            }
          })
          .catch(() => {
            // Offline / missing latest.json / dev build — do not alarm.
            pushLog("DIM", "更新检查暂不可用（需安装版 + GitHub Release 通道）");
          });
      }
    })();

    return () => {
      for (const u of unsubs) u();
      if (feedbackTimer.current) window.clearTimeout(feedbackTimer.current);
    };
  }, [pushLog, showFeedback]);

  useEffect(() => {
    if (page !== "routing") return;
    let active = true;
    const refresh = () => {
      void api
        .routingTraffic()
        .then((next) => {
          if (!active) return;
          setTraffic(next);
          setTrafficError(null);
        })
        .catch((error) => {
          if (active) setTrafficError(String(error));
        });
    };
    refresh();
    const timer = window.setInterval(refresh, 1_000);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [page]);

  const hasDefault = store.profiles.some((p) => p.id === store.default_id);
  const busy = status.busy || feedback?.state === "loading";
  // Prefer our-gateway running state; foreign port occupancy should not look "fully live"
  // for stop/restart unless health is true (user still needs a way to stop ours).
  const live = status.running || status.healthy;
  const canStop = live || status.phase === "running" || status.phase === "error";

  const onStart = () => {
    if (!hasDefault) {
      pushLog("DIM", "请先添加默认模型");
      showFeedback({ key: "start", state: "err", message: "请先添加默认模型" });
      setPage("models");
      setDialog({ mode: "add" });
      return;
    }
    beginAction("start", "正在启动网关…");
    void api.start();
  };

  const applyLiveModelConfig = async (successMessage: string) => {
    if (!live) {
      pushLog("DIM", "配置已保存，将在下次启动网关时生效");
      return;
    }
    beginAction("reload", "正在热更新上游路由…");
    try {
      const result = await api.reloadConfig();
      setStatus(result.status);
      if (!result.ok) {
        showFeedback({ key: "reload", state: "err", message: result.message });
        pushLog("ERR", result.message);
      } else {
        showFeedback({ key: "reload", state: "ok", message: successMessage });
        pushLog("OK", `${successMessage} · 客户端连接保持不变`);
      }
    } catch (e) {
      showFeedback({ key: "reload", state: "err", message: String(e) });
      pushLog("ERR", `热更新失败: ${String(e)}`);
    } finally {
      pendingKey.current = null;
    }
  };

  const saveModel = async (input: ModelInput, editId?: string) => {
    const next = editId
      ? await api.editModel(editId, input)
      : await api.createModel(input);
    setStore(next);
    setDialog(null);
    pushLog("OK", editId ? `已更新 ${input.name}` : `已保存 ${input.name}`);
    await applyLiveModelConfig(editId ? "模型配置已热更新" : "模型配置已应用");
  };

  const changeModelRouting = async (modelId: string, enabled: boolean) => {
    beginAction("routing", enabled ? `正在开启 ${modelId} 分流…` : `正在关闭 ${modelId} 分流…`);
    try {
      const next = await api.configureModelRouting(modelId, enabled);
      setStore(next);
      showFeedback({
        key: "routing",
        state: "ok",
        message: enabled ? `${modelId} 分流已开启` : `${modelId} 分流已关闭`,
      });
      pendingKey.current = null;
      pushLog("OK", enabled ? `已开启 ${modelId} 多账号分流` : `已关闭 ${modelId} 分流`);
      await applyLiveModelConfig("分流设置已热更新");
    } catch (e) {
      showFeedback({ key: "routing", state: "err", message: String(e) });
      pendingKey.current = null;
      pushLog("ERR", String(e));
    }
  };

  const changeProfileRouting = async (id: string, enabled: boolean) => {
    const profile = store.profiles.find((item) => item.id === id);
    if (!profile) return;
    beginAction("routing", enabled ? `正在启用 ${profile.name}…` : `正在停用 ${profile.name}…`);
    try {
      const next = await api.configureProfileRouting(id, enabled);
      setStore(next);
      showFeedback({
        key: "routing",
        state: "ok",
        message: enabled ? `已启用上游 ${profile.name}` : `已停用上游 ${profile.name}`,
      });
      pendingKey.current = null;
      pushLog("OK", enabled ? `分流上游已启用 · ${profile.name}` : `分流上游已停用 · ${profile.name}`);
      await applyLiveModelConfig("分流上游已热更新");
    } catch (e) {
      showFeedback({ key: "routing", state: "err", message: String(e) });
      pendingKey.current = null;
      pushLog("ERR", String(e));
    }
  };

  const navItems = useMemo(
    () => [
      { key: "gateway" as const, title: "网关", icon: Server },
      { key: "routing" as const, title: "分流预览", icon: GitBranch },
      { key: "models" as const, title: "模型", icon: Layers3 },
      { key: "clients" as const, title: "客户端", icon: Users },
      { key: "activity" as const, title: "日志", icon: Activity },
    ],
    [],
  );

  return (
    <>
      <div className="app-shell">
        <TitleBar />
        <div className="workspace">
          <SideNav
            avatar={<img src="/gateway-logo.png" width={32} height={32} alt="" style={{ borderRadius: 8 }} />}
            bottomActions={
              <Tooltip title="GitHub">
                <ActionIcon
                  icon={ExternalLink}
                  title="GitHub"
                  onClick={() => void openUrl(info?.github || GITHUB)}
                />
              </Tooltip>
            }
            topActions={
              <Flexbox gap={4}>
                {navItems.map((item) => (
                  <Tooltip key={item.key} title={item.title} placement="right">
                    <ActionIcon
                      active={page === item.key}
                      icon={item.icon}
                      size="large"
                      title={item.title}
                      onClick={() => setPage(item.key)}
                    />
                  </Tooltip>
                ))}
              </Flexbox>
            }
          />

          <main className="workspace-main">
            <div className="main-col">
              <PageHeader page={page} version={info?.version} status={status} />

              <div className="page-pane">
                {page === "gateway" && (
                  <GatewayView
                    status={status}
                    store={store}
                    busy={busy}
                    live={live}
                    canStop={canStop}
                    feedback={feedback}
                    recentLogs={logs.slice(-12)}
                    onStart={onStart}
                    onStop={() => {
                      beginAction("stop", "正在停止网关…");
                      void api.stop().catch((e) => {
                        showFeedback({ key: "stop", state: "err", message: String(e) });
                        pendingKey.current = null;
                      });
                    }}
                    onRestart={() => {
                      if (!hasDefault) {
                        setPage("models");
                        showFeedback({
                          key: "restart",
                          state: "err",
                          message: "请先配置默认模型",
                        });
                        return;
                      }
                      beginAction("restart", "正在重启网关…");
                      void api.restart().catch((e) => {
                        showFeedback({ key: "restart", state: "err", message: String(e) });
                        pendingKey.current = null;
                      });
                    }}
                    onCheck={() => {
                      void (async () => {
                        beginAction("check", "正在健康检查…");
                        try {
                          const r = await api.check();
                          setStatus(r.status);
                          showFeedback({
                            key: "check",
                            state: r.ok ? "ok" : "err",
                            message: r.ok
                              ? "健康检查通过"
                              : r.message || "健康检查失败",
                          });
                          pendingKey.current = null;
                        } catch (e) {
                          showFeedback({
                            key: "check",
                            state: "err",
                            message: String(e),
                          });
                          pendingKey.current = null;
                        }
                      })();
                    }}
                    onLogs={async () => {
                      beginAction("logs", "打开日志目录…");
                      try {
                        const dir = await api.openLogsDir();
                        showFeedback({ key: "logs", state: "ok", message: "已打开日志目录" });
                        pendingKey.current = null;
                        pushLog("OK", `日志目录 ${dir}`);
                      } catch (e) {
                        showFeedback({ key: "logs", state: "err", message: String(e) });
                        pendingKey.current = null;
                        pushLog("ERR", String(e));
                      }
                    }}
                    onUi={() => {
                      const base = (status.endpoint || "http://127.0.0.1:4000/v1").replace(
                        /\/v1\/?$/,
                        "",
                      );
                      void openUrl(`${base}/ui`);
                    }}
                  />
                )}

                {page === "models" && (
                  <ModelsView
                    store={store}
                    selectedId={selectedId}
                    busy={busy}
                    feedback={feedback}
                    onSelect={setSelectedId}
                    onAdd={() => setDialog({ mode: "add" })}
                    onImport={() => {
                      void (async () => {
                        try {
                          const picked = await open({
                            multiple: false,
                            filters: [
                              { name: "模型配置", extensions: ["txt", "env", "conf", "ini"] },
                              { name: "全部文件", extensions: ["*"] },
                            ],
                            title: "导入模型配置（api.txt）",
                          });
                          if (!picked || Array.isArray(picked)) return;
                          beginAction("import", "正在解析配置…");
                          const parsed = await api.parseModelFile(picked);
                          const models = await resolveImportModels(parsed, confirm, pushLog);
                          if (!models || models.length === 0) {
                            showFeedback({
                              key: "import",
                              state: "err",
                              message: "未选择任何模型",
                            });
                            pendingKey.current = null;
                            return;
                          }
                          const next = await api.importModelProfiles(
                            parsed.base_url,
                            parsed.api_key,
                            models,
                            parsed.name_hint,
                          );
                          setStore(next);
                          showFeedback({
                            key: "import",
                            state: "ok",
                            message: `已导入 ${models.length} 个模型`,
                          });
                          pendingKey.current = null;
                          pushLog(
                            "OK",
                            `已从文本导入 ${models.length} 个模型 · ${parsed.base_url}`,
                          );
                          await applyLiveModelConfig("导入的模型配置已应用");
                        } catch (e) {
                          showFeedback({
                            key: "import",
                            state: "err",
                            message: String(e),
                          });
                          pendingKey.current = null;
                          pushLog("ERR", `导入失败: ${String(e)}`);
                        }
                      })();
                    }}
                    onModelRoutingChange={(modelId, enabled) =>
                      void changeModelRouting(modelId, enabled)
                    }
                    onProfileRoutingChange={(id, enabled) =>
                      void changeProfileRouting(id, enabled)
                    }
                    onEdit={() => {
                      const id =
                        selectedId || store.default_id || store.profiles[0]?.id || null;
                      const p = store.profiles.find((x) => x.id === id);
                      if (!p) {
                        pushLog("DIM", "请先选择模型");
                        return;
                      }
                      setDialog({ mode: "edit", profile: p });
                    }}
                    onDefault={async () => {
                      const id =
                        selectedId || store.default_id || store.profiles[0]?.id || null;
                      if (!id) return;
                      if (id === store.default_id) {
                        showFeedback({
                          key: "default",
                          state: "ok",
                          message:
                            store.profiles.length === 1
                              ? "当前唯一配置已是默认"
                              : "已是默认模型",
                        });
                        return;
                      }
                      beginAction("default", "正在设为默认…");
                      try {
                        const next = await api.makeDefault(id);
                        setStore(next);
                        setSelectedId(id);
                        showFeedback({ key: "default", state: "ok", message: "已设为默认模型" });
                        pendingKey.current = null;
                        pushLog("OK", "已设为默认");
                        await applyLiveModelConfig("默认模型已即时切换");
                      } catch (e) {
                        showFeedback({ key: "default", state: "err", message: String(e) });
                        pendingKey.current = null;
                        pushLog("ERR", String(e));
                      }
                    }}
                    onDelete={async () => {
                      const id =
                        selectedId || store.default_id || store.profiles[0]?.id || null;
                      if (!id) return;
                      const p = store.profiles.find((x) => x.id === id);
                      if (!p) return;
                      const ok = await confirm({
                        title: "确认删除",
                        content: `删除模型配置「${p.name}」？此操作不可撤销。`,
                        okText: "删除",
                        cancelText: "取消",
                        danger: true,
                      });
                      if (!ok) return;
                      beginAction("delete", "正在删除…");
                      try {
                        const next = await api.removeModel(id);
                        setStore(next);
                        showFeedback({ key: "delete", state: "ok", message: `已删除 ${p.name}` });
                        pendingKey.current = null;
                        pushLog("OK", `已删除 ${p.name}`);
                        if (live && next.profiles.length === 0) {
                          beginAction("stop", "最后一个模型已删除，正在停止网关…");
                          void api.stop();
                        } else {
                          await applyLiveModelConfig("模型删除已即时应用");
                        }
                      } catch (e) {
                        showFeedback({ key: "delete", state: "err", message: String(e) });
                        pendingKey.current = null;
                        pushLog("ERR", String(e));
                      }
                    }}
                    onRefresh={() =>
                      void api
                        .listModels()
                        .then((m) => {
                          setStore(m);
                          pushLog("OK", "模型列表已刷新");
                        })
                        .catch((e) => pushLog("ERR", `刷新失败: ${String(e)}`))
                    }
                  />
                )}

                {page === "routing" && (
                  <RoutingMapView
                    store={store}
                    traffic={traffic}
                    status={status}
                    error={trafficError}
                  />
                )}

                {page === "clients" && (
                  <ClientsView
                    busy={busy}
                    autostart={!!info?.autostart}
                    version={info?.version}
                    feedback={feedback}
                    updateProgress={updateProgress}
                    onScript={(key, label, script, confirmText) => {
                      void (async () => {
                        if (confirmText) {
                          const ok = await confirm({
                            title: label,
                            content: confirmText,
                            okText: "继续",
                            cancelText: "取消",
                            danger: key.includes("restore"),
                          });
                          if (!ok) {
                            pushLog("DIM", `已取消 · ${label}`);
                            return;
                          }
                        }
                        beginAction(key, `${label}…`);
                        pushLog("INFO", `▶ ${label}`);
                        try {
                          const r = await api.runScript(script);
                          setStatus(r.status);
                          setInfo(await api.projectInfo());
                          showFeedback({
                            key,
                            state: r.ok ? "ok" : "err",
                            message: r.ok ? `${label}完成` : r.message || `${label}失败`,
                          });
                          pendingKey.current = null;
                        } catch (e) {
                          showFeedback({ key, state: "err", message: String(e) });
                          pendingKey.current = null;
                          pushLog("ERR", String(e));
                        }
                      })();
                    }}
                    onAutostart={() => {
                      const enable = !info?.autostart;
                      void (async () => {
                        beginAction("autostart", enable ? "正在启用自启…" : "正在关闭自启…");
                        try {
                          const msg = await api.toggleAutostart(enable);
                          showFeedback({ key: "autostart", state: "ok", message: msg || "已更新" });
                          pendingKey.current = null;
                          pushLog("OK", msg);
                          setInfo(await api.projectInfo());
                        } catch (e) {
                          showFeedback({ key: "autostart", state: "err", message: String(e) });
                          pendingKey.current = null;
                          pushLog("ERR", String(e));
                        }
                      })();
                    }}
                    onCheckUpdate={() => {
                      void (async () => {
                        beginAction("update", "正在检查更新…");
                        setUpdateProgress({
                          phase: "checking",
                          downloaded: 0,
                          total: null,
                          message: "正在连接 GitHub Release…",
                        });
                        try {
                          const update = await checkUpdate();
                          if (!update) {
                            setUpdateProgress(null);
                            showFeedback({
                              key: "update",
                              state: "ok",
                              message: "已是最新版本",
                            });
                            pendingKey.current = null;
                            pushLog("OK", "已是最新版本");
                            return;
                          }
                          const notes = (update.body || "").trim();
                          const yes = await confirm({
                            title: `发现新版本 ${update.version}`,
                            content:
                              `当前 ${update.currentVersion} → ${update.version}\n\n` +
                              (notes ? `${notes.slice(0, 600)}\n\n` : "") +
                              "将下载完整 Studio 安装包（SHA-256 校验）并覆盖安装。\n" +
                              "不会修改 .gateway 模型密钥；网关进程不会因此被停止。\n" +
                              "安装器启动后控制台会退出，按向导完成安装即可。",
                            okText: "下载并安装",
                            cancelText: "稍后",
                          });
                          if (!yes) {
                            setUpdateProgress(null);
                            showFeedback({
                              key: "update",
                              state: "ok",
                              message: "已取消更新",
                            });
                            pendingKey.current = null;
                            await update.close().catch(() => undefined);
                            return;
                          }
                          pushLog("INFO", `▶ 更新至 ${update.version}`);
                          await installKnownUpdate(update, (p) => {
                            setUpdateProgress(p);
                            if (p.phase === "downloading" || p.phase === "installing") {
                              showFeedback({
                                key: "update",
                                state: "loading",
                                message: p.message,
                              });
                            }
                          });
                        } catch (e) {
                          setUpdateProgress(null);
                          const msg = String(e);
                          showFeedback({ key: "update", state: "err", message: msg });
                          pendingKey.current = null;
                          pushLog("ERR", `更新失败: ${msg}`);
                        }
                      })();
                    }}
                  />
                )}

                {page === "activity" && (
                  <ActivityView
                    logs={logs}
                    onClear={() => setLogs([])}
                    onCopy={() => {
                      const text = logs.map((l) => `${l.level}  ${l.message}`).join("\n");
                      void copyText(text).then((ok) =>
                        pushLog(ok ? "OK" : "ERR", ok ? "已复制日志" : "复制失败"),
                      );
                    }}
                  />
                )}
              </div>

              <CreditBar info={info} />
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
              setNotice({
                title: "保存失败",
                content: String(e),
                type: "error",
              });
              throw e;
            }
          }}
        />
      )}

      <AppConfirmModal
        request={confirmReq}
        onClose={(ok) => {
          const req = confirmReq;
          setConfirmReq(null);
          req?.resolve(ok);
        }}
      />

      <AppNoticeModal
        notice={notice}
        onClose={() => setNotice(null)}
      />
    </>
  );
}

function AppConfirmModal({
  request,
  onClose,
}: {
  request: ConfirmRequest | null;
  onClose: (ok: boolean) => void;
}) {
  if (!request) return null;
  return (
    <Modal
      open
      title={request.title}
      onCancel={() => onClose(false)}
      footer={
        <Flexbox horizontal gap={8} distribution="flex-end">
          <Button onClick={() => onClose(false)}>{request.cancelText || "取消"}</Button>
          <Button
            type="primary"
            danger={request.danger}
            onClick={() => onClose(true)}
          >
            {request.okText || "确定"}
          </Button>
        </Flexbox>
      }
      width={420}
      destroyOnClose
    >
      <Flexbox horizontal gap={12} align="flex-start">
        <Icon
          icon={AlertTriangle}
          size="large"
          style={{ color: request.danger ? "#ff6b7a" : "#f4d28a", marginTop: 2 }}
        />
        <Text style={{ lineHeight: 1.6 }}>{request.content}</Text>
      </Flexbox>
    </Modal>
  );
}

function AppNoticeModal({
  notice,
  onClose,
}: {
  notice: NoticeRequest | null;
  onClose: () => void;
}) {
  if (!notice) return null;
  return (
    <Modal
      open
      title={notice.title}
      onCancel={onClose}
      footer={
        <Flexbox horizontal distribution="flex-end">
          <Button type="primary" onClick={onClose}>
            知道了
          </Button>
        </Flexbox>
      }
      width={420}
      destroyOnClose
    >
      <Alert
        type={notice.type === "error" ? "error" : notice.type === "success" ? "success" : "info"}
        showIcon
        message={notice.content}
      />
    </Modal>
  );
}

const PageHeader = memo(function PageHeader({
  page,
  version,
  status,
}: {
  page: Page;
  version?: string;
  status: GatewayStatus;
}) {
  const meta = {
    gateway: { title: "网关运行时", sub: "本机 LiteLLM 进程 · 仅 127.0.0.1" },
    routing: { title: "分流预览", sub: "真实选路轨迹 · 模型 → 上游网站" },
    models: { title: "上游模型", sub: "密钥保存在 .gateway/models.json" },
    clients: { title: "客户端接入", sub: "Codex / Claude Desktop · 可安全恢复" },
    activity: { title: "运行日志", sub: "后端事件流 · 脚本输出" },
  }[page];

  const phaseColor =
    status.phase === "running"
      ? "success"
      : status.phase === "error"
        ? "error"
        : status.phase === "starting" || status.phase === "stopping"
          ? "warning"
          : "default";

  return (
    <Flexbox horizontal align="flex-start" distribution="space-between" gap={12}>
      <Flexbox gap={4}>
        <Text type="secondary" fontSize={12} weight={600} style={{ letterSpacing: "0.12em" }}>
          {page.toUpperCase()}
        </Text>
        <Text as="h2" fontSize={24} weight={700} style={{ margin: 0 }}>
          {meta.title}
        </Text>
        <Text type="secondary" fontSize={13}>
          {meta.sub}
        </Text>
      </Flexbox>
      <Flexbox horizontal gap={8} align="center">
        <Tag color={phaseColor}>{status.phase}</Tag>
        {version && <Tag>{version}</Tag>}
      </Flexbox>
    </Flexbox>
  );
});

function FeedbackAlert({ feedback }: { feedback: Feedback | null }) {
  if (!feedback) return null;
  const type =
    feedback.state === "loading" ? "info" : feedback.state === "ok" ? "success" : "error";
  return (
    <Alert
      type={type}
      showIcon
      message={
        feedback.state === "loading"
          ? feedback.message
          : feedback.state === "ok"
            ? `✓ ${feedback.message}`
            : `✗ ${feedback.message}`
      }
    />
  );
}

function isLoading(feedback: Feedback | null, key: ActionKey) {
  return feedback?.state === "loading" && feedback.key === key;
}

function GatewayView({
  status,
  store,
  busy,
  live,
  canStop,
  feedback,
  recentLogs,
  onStart,
  onStop,
  onRestart,
  onCheck,
  onLogs,
  onUi,
}: {
  status: GatewayStatus;
  store: ModelStore;
  busy: boolean;
  live: boolean;
  canStop: boolean;
  feedback: Feedback | null;
  recentLogs: LogLine[];
  onStart: () => void;
  onStop: () => void;
  onRestart: () => void;
  onCheck: () => void;
  onLogs: () => void;
  onUi: () => void;
}) {
  const defaultName =
    status.default_model_name ||
    store.profiles.find((p) => p.id === store.default_id)?.name ||
    "未配置";
  const logRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    logRef.current?.scrollTo({ top: logRef.current.scrollHeight });
  }, [recentLogs]);

  const statusLabel =
    status.phase === "stopping"
      ? "停止中…"
      : status.phase === "starting"
        ? "启动中…"
        : live
          ? status.is_our_gateway
            ? "运行中"
            : "端口占用"
          : "已停止";

  return (
    <Flexbox gap={12} style={{ height: "100%", minHeight: 0 }}>
      {!status.is_our_gateway && status.healthy && (
        <Alert
          type="warning"
          showIcon
          message="端口 4000 有响应，但可能不是本网关（缺少 codex-chat 路由）"
        />
      )}

      <Flexbox horizontal gap={12} style={{ flexWrap: "wrap", flexShrink: 0 }}>
        <Block variant="outlined" padding={16} style={{ flex: "1 1 200px", minWidth: 180 }}>
          <Text type="secondary" fontSize={12}>
            状态
          </Text>
          <Text fontSize={22} weight={700} style={{ marginTop: 8 }}>
            {statusLabel}
          </Text>
          <Text type="secondary" fontSize={12} style={{ marginTop: 8 }}>
            {status.message}
          </Text>
        </Block>
        <Block variant="outlined" padding={16} style={{ flex: "1 1 200px", minWidth: 180 }}>
          <Text type="secondary" fontSize={12}>
            进程
          </Text>
          <Text fontSize={22} weight={700} style={{ marginTop: 8 }}>
            {status.pid ?? "—"}
          </Text>
          <Text type="secondary" fontSize={12} style={{ marginTop: 8 }}>
            {status.uptime ? `已运行 ${status.uptime}` : "启动后显示运行时长"}
          </Text>
        </Block>
        <Block variant="outlined" padding={16} style={{ flex: "1 1 200px", minWidth: 180 }}>
          <Text type="secondary" fontSize={12}>
            默认模型
          </Text>
          <Text fontSize={18} weight={700} style={{ marginTop: 8 }}>
            {defaultName}
          </Text>
          <Text type="secondary" fontSize={12} style={{ marginTop: 8 }} ellipsis>
            {status.model || "—"}
          </Text>
        </Block>
      </Flexbox>

      {status.phase === "starting" && (
        <div
          className="gateway-startup-progress"
          role="progressbar"
          aria-label="网关启动进度"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={status.startup_progress ?? 4}
        >
          <div className="gateway-startup-progress-head">
            <div>
              <span className="gateway-startup-kicker">BOOT SEQUENCE</span>
              <strong>{status.startup_stage || "正在启动网关"}</strong>
            </div>
            <span className="gateway-startup-percent">
              {Math.max(0, Math.min(100, status.startup_progress ?? 4))}%
            </span>
          </div>
          <div className="gateway-startup-track" aria-hidden="true">
            <span
              className="gateway-startup-fill"
              style={{ width: `${Math.max(4, Math.min(100, status.startup_progress ?? 4))}%` }}
            />
          </div>
          <div className="gateway-startup-note">
            首次启动需要加载本地 Python 与 LiteLLM，后续启动会更快。请保持窗口开启。
          </div>
        </div>
      )}

      <Block variant="outlined" padding={16} style={{ flexShrink: 0 }}>
        <Flexbox gap={12}>
          <Flexbox horizontal gap={10} align="center" style={{ flexWrap: "wrap" }}>
            <Text type="secondary" fontSize={12}>
              Endpoint
            </Text>
            <Button
              size="small"
              icon={Copy}
              onClick={() => void copyText(status.endpoint)}
              title="复制本地接口地址"
            >
              {status.endpoint}
            </Button>
          </Flexbox>
          <Flexbox horizontal gap={8} style={{ flexWrap: "wrap" }}>
            <Button
              type="primary"
              icon={Play}
              loading={isLoading(feedback, "start") || status.phase === "starting"}
              disabled={busy || (live && status.is_our_gateway)}
              onClick={onStart}
            >
              启动
            </Button>
            <Button
              icon={Square}
              loading={isLoading(feedback, "stop") || status.phase === "stopping"}
              disabled={status.busy || status.phase === "stopping" || !canStop}
              onClick={onStop}
            >
              停止
            </Button>
            <Button
              icon={RotateCcw}
              loading={isLoading(feedback, "restart")}
              disabled={busy || !live}
              onClick={onRestart}
            >
              重启
            </Button>
            <Button
              icon={CheckCircle2}
              loading={isLoading(feedback, "check")}
              disabled={busy}
              onClick={onCheck}
            >
              健康检查
            </Button>
            <Button
              icon={FolderOpen}
              variant="outlined"
              loading={isLoading(feedback, "logs")}
              disabled={busy}
              onClick={onLogs}
            >
              日志目录
            </Button>
            <Button icon={Bot} variant="outlined" onClick={onUi}>
              LiteLLM UI
            </Button>
          </Flexbox>
          <FeedbackAlert
            feedback={
              feedback &&
              ["start", "stop", "restart", "check", "logs"].includes(feedback.key)
                ? feedback
                : null
            }
          />
        </Flexbox>
      </Block>

      <Block
        variant="outlined"
        padding={0}
        style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}
      >
        <Flexbox
          horizontal
          distribution="space-between"
          align="center"
          padding={12}
          style={{ borderBottom: "1px solid rgba(255,255,255,0.06)", flexShrink: 0 }}
        >
          <Text weight={600} fontSize={13}>
            最近操作
          </Text>
          <Text type="secondary" fontSize={11}>
            完整日志见左侧「日志」页
          </Text>
        </Flexbox>
        <div className="mini-log" ref={logRef}>
          {recentLogs.length === 0 ? (
            <Text type="secondary">启动 / 停止 / 检查后的结果会出现在这里</Text>
          ) : (
            recentLogs.map((l) => (
              <div className="log-line" key={l.id}>
                <span style={{ opacity: 0.7, fontWeight: 700 }}>{l.level}</span>
                <span>{l.message}</span>
              </div>
            ))
          )}
        </div>
      </Block>
    </Flexbox>
  );
}

async function resolveImportModels(
  parsed: ParsedApiText,
  confirm: (opts: Omit<ConfirmRequest, "resolve">) => Promise<boolean>,
  pushLog: (level: LogLevel, msg: string) => void,
): Promise<string[] | null> {
  if (parsed.models.length > 0) {
    pushLog("DIM", `识别到 ${parsed.models.length} 个模型 ID`);
    return parsed.models;
  }
  const yes = await confirm({
    title: "未检测到模型",
    content:
      "配置文件中 model 为空或未填写。是否在线拉取模型列表？\n\n" +
      `接口：${parsed.base_url}`,
    okText: "在线拉取",
    cancelText: "取消导入",
  });
  if (!yes) return null;
  pushLog("INFO", `▶ 在线拉取模型 · ${parsed.base_url}`);
  try {
    const ids = await api.fetchModels(parsed.base_url, parsed.api_key);
    if (ids.length === 0) {
      pushLog("ERR", "在线列表为空");
      return null;
    }
    // Import all listed models; user can delete unused later.
    // Cap very large catalogs to keep store manageable.
    const capped = ids.slice(0, 40);
    if (ids.length > capped.length) {
      pushLog("DIM", `接口返回 ${ids.length} 个模型，仅导入前 ${capped.length} 个`);
    }
    const ok = await confirm({
      title: "确认导入",
      content: `将导入 ${capped.length} 个模型到本地配置（可稍后删除/设默认）。`,
      okText: "导入",
      cancelText: "取消",
    });
    return ok ? capped : null;
  } catch (e) {
    pushLog("ERR", String(e));
    throw e;
  }
}

function normalizedModelId(value: string) {
  const normalized = value.trim().toLowerCase();
  const [provider, ...rest] = normalized.split("/");
  return rest.length > 0 && ["openai", "custom_openai", "deepseek"].includes(provider)
    ? rest.join("/")
    : normalized;
}

function isDefaultModelId(store: ModelStore, modelId: string) {
  const current =
    store.profiles.find((item) => item.id === store.default_id) ?? store.profiles[0];
  return !!current && normalizedModelId(current.model_id) === normalizedModelId(modelId);
}

function isModelRoutingEnabled(store: ModelStore, modelId: string) {
  const rules = store.routing.model_rules ?? [];
  if (rules.length === 0) {
    return store.routing.enabled && isDefaultModelId(store, modelId);
  }
  return !!rules.find(
    (rule) => normalizedModelId(rule.model_id) === normalizedModelId(modelId),
  )?.enabled;
}

function upstreamHost(baseUrl: string) {
  try {
    return new URL(baseUrl).host;
  } catch {
    return baseUrl;
  }
}

function ModelsView({
  store,
  selectedId,
  busy,
  feedback,
  onSelect,
  onAdd,
  onImport,
  onModelRoutingChange,
  onProfileRoutingChange,
  onEdit,
  onDefault,
  onDelete,
  onRefresh,
}: {
  store: ModelStore;
  selectedId: string | null;
  busy: boolean;
  feedback: Feedback | null;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onImport: () => void;
  onModelRoutingChange: (modelId: string, enabled: boolean) => void;
  onProfileRoutingChange: (id: string, enabled: boolean) => void;
  onEdit: () => void;
  onDefault: () => void;
  onDelete: () => void;
  onRefresh: () => void;
}) {
  const activeId = selectedId || store.default_id || store.profiles[0]?.id || null;
  const activeIsDefault = !!activeId && activeId === store.default_id;
  const onlyOne = store.profiles.length === 1;
  const modelGroups = useMemo(() => {
    const grouped = new Map<string, { modelId: string; profiles: ModelProfile[] }>();
    for (const profile of store.profiles) {
      const key = normalizedModelId(profile.model_id);
      const group = grouped.get(key) ?? { modelId: profile.model_id, profiles: [] };
      group.profiles.push(profile);
      grouped.set(key, group);
    }
    return [...grouped.entries()]
      .map(([key, group]) => ({ key, ...group }))
      .sort((a, b) => {
        const aDefault = isDefaultModelId(store, a.modelId) ? 1 : 0;
        const bDefault = isDefaultModelId(store, b.modelId) ? 1 : 0;
        return bDefault - aDefault || b.profiles.length - a.profiles.length || a.modelId.localeCompare(b.modelId);
      });
  }, [store]);
  const enabledModelCount = modelGroups.filter((group) =>
    isModelRoutingEnabled(store, group.modelId),
  ).length;

  return (
    <Flexbox gap={14} style={{ height: "100%" }}>
      <Flexbox horizontal distribution="space-between" align="center">
        <Text type="secondary">{store.profiles.length} 个配置</Text>
        <Flexbox horizontal gap={8}>
          <Button type="primary" icon={Plus} disabled={busy} onClick={onAdd}>
            添加模型
          </Button>
          <Button disabled={busy} loading={isLoading(feedback, "import")} onClick={onImport}>
            导入 txt
          </Button>
          <Button icon={RefreshCw} disabled={busy} onClick={onRefresh}>
            刷新
          </Button>
        </Flexbox>
      </Flexbox>

      {store.profiles.length > 0 && (
        <section className="routing-console">
          <Flexbox horizontal distribution="space-between" align="flex-end" gap={16}>
            <Flexbox gap={3}>
              <Text weight={800}>分流模型管理</Text>
              <Text type="secondary" fontSize={12}>
                按模型绑定上游；新会话按权重分配，已有会话保持亲和约 {Math.round((store.routing.affinity_ttl_seconds || 3600) / 60)} 分钟
              </Text>
            </Flexbox>
            <Tag color={enabledModelCount > 0 ? "success" : "default"}>
              {enabledModelCount} / {modelGroups.length} 已开启
            </Tag>
          </Flexbox>

          <div className="routing-groups">
            {modelGroups.map((group) => {
              const groupEnabled = isModelRoutingEnabled(store, group.modelId);
              const enabledProfiles = group.profiles.filter((profile) => profile.routing_enabled);
              const defaultGroup = isDefaultModelId(store, group.modelId);
              return (
                <Block
                  key={group.key}
                  className={`routing-group${groupEnabled ? " is-enabled" : ""}${defaultGroup ? " is-default" : ""}`}
                  variant="outlined"
                  padding={0}
                >
                  <div className="routing-group-head">
                    <Flexbox gap={4} style={{ minWidth: 0 }}>
                      <Flexbox horizontal gap={7} align="center" style={{ minWidth: 0 }}>
                        <span className="routing-status-dot" />
                        <Text weight={800} ellipsis>{group.modelId}</Text>
                        {defaultGroup && <Tag color="success">当前默认</Tag>}
                      </Flexbox>
                      <Text type="secondary" fontSize={11}>
                        {group.profiles.length} 个上游 · {enabledProfiles.length} 个已选择
                      </Text>
                    </Flexbox>
                    <Flexbox horizontal gap={10} align="center">
                      <Text type="secondary" fontSize={11}>
                        {groupEnabled ? "模型分流已开启" : "模型分流已关闭"}
                      </Text>
                      <Switch
                        checked={groupEnabled}
                        loading={isLoading(feedback, "routing")}
                        disabled={busy}
                        onChange={(enabled) => onModelRoutingChange(group.modelId, enabled)}
                      />
                    </Flexbox>
                  </div>

                  <div className="routing-upstreams">
                    {group.profiles.map((profile) => {
                      const isDefault = profile.id === store.default_id;
                      return (
                        <div className="routing-upstream" key={profile.id}>
                          <span className={`upstream-rail${profile.routing_enabled ? " is-on" : ""}`} />
                          <Flexbox gap={2} style={{ minWidth: 0, flex: 1 }}>
                            <Flexbox horizontal gap={6} align="center" style={{ minWidth: 0 }}>
                              <Text weight={700} fontSize={12} ellipsis>{profile.name}</Text>
                              {isDefault && <Tag>默认上游</Tag>}
                            </Flexbox>
                            <Text type="secondary" fontSize={10.5} ellipsis>
                              {upstreamHost(profile.base_url)}
                            </Text>
                          </Flexbox>
                          <Tag color={profile.routing_enabled ? "blue" : "default"}>
                            权重 × {profile.routing_weight}
                          </Tag>
                          <Switch
                            size="small"
                            checked={profile.routing_enabled}
                            disabled={busy}
                            onChange={(enabled) => onProfileRoutingChange(profile.id, enabled)}
                          />
                        </div>
                      );
                    })}
                  </div>

                  {groupEnabled && enabledProfiles.length < 2 && (
                    <div className="routing-group-note">
                      当前只有 {enabledProfiles.length} 个上游启用；至少启用 2 个才会产生实际分流。
                    </div>
                  )}
                </Block>
              );
            })}
          </div>
        </section>
      )}

      {store.profiles.length === 0 ? (
        <>
          <Empty
            title="还没有上游模型"
            description="手动添加，或导入 baseurl/key/model 文本配置"
            action={
              <Flexbox horizontal gap={8}>
                <Button type="primary" icon={Plus} onClick={onAdd}>
                  立即添加
                </Button>
                <Button
                  loading={isLoading(feedback, "import")}
                  disabled={busy}
                  onClick={onImport}
                >
                  导入 txt
                </Button>
              </Flexbox>
            }
          />
          <FeedbackAlert
            feedback={feedback && feedback.key === "import" ? feedback : null}
          />
        </>
      ) : (
        <>
          <Flexbox horizontal distribution="space-between" align="center">
            <Text weight={700}>上游配置</Text>
            <Text type="secondary" fontSize={11}>双击卡片可编辑地址、密钥与权重</Text>
          </Flexbox>
          <Flexbox gap={10} horizontal style={{ flexWrap: "wrap" }}>
            {store.profiles.map((p) => {
              const isDefault = p.id === store.default_id;
              const selected = activeId === p.id;
              return (
                <Block
                  key={p.id}
                  clickable
                  variant={selected ? "filled" : "outlined"}
                  padding={14}
                  onClick={() => onSelect(p.id)}
                  onDoubleClick={onEdit}
                  style={{ width: 280, cursor: "pointer" }}
                >
                  <Flexbox gap={8}>
                    <Flexbox horizontal distribution="space-between" align="center">
                      <Text weight={700} ellipsis style={{ maxWidth: 180 }}>
                        {p.name}
                      </Text>
                      {isDefault && <Tag color="success">DEFAULT</Tag>}
                    </Flexbox>
                    <Text type="secondary" fontSize={12} ellipsis>
                      {p.model_id}
                    </Text>
                    <Text type="secondary" fontSize={11} ellipsis>
                      {p.litellm_model}
                    </Text>
                    <Text type="secondary" fontSize={11} ellipsis>
                      {p.base_url}
                    </Text>
                    <Flexbox horizontal gap={6} align="center">
                      <Tag color={p.routing_enabled && isModelRoutingEnabled(store, p.model_id) ? "blue" : "default"}>
                        {!p.routing_enabled
                          ? "上游已停用"
                          : isModelRoutingEnabled(store, p.model_id)
                            ? `分流 × ${p.routing_weight}`
                            : `候选 × ${p.routing_weight}`}
                      </Tag>
                    </Flexbox>
                  </Flexbox>
                </Block>
              );
            })}
          </Flexbox>
          <Flexbox horizontal gap={8} align="center" style={{ flexWrap: "wrap" }}>
            <Tooltip
              title={
                activeIsDefault
                  ? onlyOne
                    ? "当前唯一配置已是默认"
                    : "已是默认模型"
                  : "将选中项设为默认"
              }
            >
              <Button
                icon={Star}
                loading={isLoading(feedback, "default")}
                disabled={busy || !activeId}
                onClick={onDefault}
              >
                {activeIsDefault ? "已是默认" : "设为默认"}
              </Button>
            </Tooltip>
            <Button
              icon={Pencil}
              disabled={busy || !activeId}
              onClick={onEdit}
            >
              编辑
            </Button>
            <Button
              danger
              icon={Trash2}
              loading={isLoading(feedback, "delete")}
              disabled={busy || !activeId}
              onClick={onDelete}
            >
              删除
            </Button>
          </Flexbox>
          <FeedbackAlert
            feedback={
              feedback && ["default", "delete", "import", "routing"].includes(feedback.key)
                ? feedback
                : null
            }
          />
        </>
      )}
    </Flexbox>
  );
}

function ClientsView({
  busy,
  autostart,
  version,
  feedback,
  updateProgress,
  onScript,
  onAutostart,
  onCheckUpdate,
}: {
  busy: boolean;
  autostart: boolean;
  version?: string;
  feedback: Feedback | null;
  updateProgress: UpdateProgress | null;
  onScript: (
    key: ActionKey,
    label: string,
    script: string,
    confirm?: string,
  ) => void;
  onAutostart: () => void;
  onCheckUpdate: () => void;
}) {
  const updateLoading = isLoading(feedback, "update");
  const pct =
    updateProgress?.total && updateProgress.total > 0
      ? Math.min(100, Math.round((updateProgress.downloaded / updateProgress.total) * 100))
      : null;

  return (
    <Flexbox gap={12} style={{ height: "100%" }}>
      <Flexbox horizontal gap={12} style={{ flexWrap: "wrap" }}>
        <Block variant="outlined" padding={18} style={{ flex: "1 1 320px" }}>
          <Flexbox gap={10}>
            <Text weight={700} fontSize={16}>
              Codex
            </Text>
            <Text type="secondary" fontSize={13}>
              写入 Responses 提供方，自动备份并保留 MCP / 插件。
            </Text>
            <Flexbox horizontal gap={8}>
              <Button
                type="primary"
                loading={isLoading(feedback, "codex-cfg")}
                disabled={busy}
                onClick={() => onScript("codex-cfg", "配置 Codex", "configure-codex.ps1")}
              >
                配置
              </Button>
              <Button
                danger
                loading={isLoading(feedback, "codex-restore")}
                disabled={busy}
                onClick={() =>
                  onScript(
                    "codex-restore",
                    "恢复 Codex",
                    "restore-codex.ps1",
                    "撤销网关相关 Codex 配置并尽量恢复官方设置？",
                  )
                }
              >
                恢复官方
              </Button>
            </Flexbox>
          </Flexbox>
        </Block>

        <Block variant="outlined" padding={18} style={{ flex: "1 1 320px" }}>
          <Flexbox gap={10}>
            <Text weight={700} fontSize={16}>
              Claude Desktop
            </Text>
            <Text type="secondary" fontSize={13}>
              仅配置 Code 模式 3P Profile，不改普通聊天或 MCP。
            </Text>
            <Flexbox horizontal gap={8}>
              <Button
                type="primary"
                loading={isLoading(feedback, "claude-cfg")}
                disabled={busy}
                onClick={() =>
                  onScript(
                    "claude-cfg",
                    "配置 Claude Desktop",
                    "configure-claude-desktop.ps1",
                  )
                }
              >
                配置
              </Button>
              <Button
                danger
                loading={isLoading(feedback, "claude-restore")}
                disabled={busy}
                onClick={() =>
                  onScript(
                    "claude-restore",
                    "恢复 Claude",
                    "restore-claude-desktop.ps1",
                    "移除本项目 Profile 并切回官方 1P 模式？",
                  )
                }
              >
                恢复官方
              </Button>
            </Flexbox>
          </Flexbox>
        </Block>
      </Flexbox>

      <Block variant="outlined" padding={18}>
        <Flexbox horizontal distribution="space-between" align="center" gap={12}>
          <Flexbox gap={4}>
            <Text weight={700} fontSize={16}>
              登录自启
            </Text>
            <Text type="secondary" fontSize={13}>
              登录 Windows 后自动启动网关进程（不启动本控制台）
            </Text>
          </Flexbox>
          <Button
            icon={Settings2}
            loading={isLoading(feedback, "autostart")}
            disabled={busy}
            onClick={onAutostart}
          >
            {autostart ? "已开启 · 点击关闭" : "已关闭 · 点击开启"}
          </Button>
        </Flexbox>
      </Block>

      <Block variant="outlined" padding={18}>
        <Flexbox gap={12}>
          <Flexbox horizontal distribution="space-between" align="center" gap={12}>
            <Flexbox gap={4}>
              <Text weight={700} fontSize={16}>
                自动更新
              </Text>
              <Text type="secondary" fontSize={13}>
                HTTPS GitHub Release · 签名校验 · 当前 {version || "—"}
              </Text>
            </Flexbox>
            <Button
              type="primary"
              icon={Download}
              loading={updateLoading}
              disabled={busy && !updateLoading}
              onClick={onCheckUpdate}
            >
              检查更新
            </Button>
          </Flexbox>
          {updateProgress && (
            <Alert
              type={updateProgress.phase === "done" ? "success" : "info"}
              showIcon
              message={
                pct != null
                  ? `${updateProgress.message} · ${pct}%`
                  : updateProgress.message
              }
            />
          )}
          <Text type="secondary" fontSize={12}>
            仅更新 Studio 控制台安装包；不会改写 .gateway 模型配置，也不会因更新自动停止网关。
          </Text>
        </Flexbox>
      </Block>

      <FeedbackAlert
        feedback={
          feedback &&
          [
            "codex-cfg",
            "codex-restore",
            "claude-cfg",
            "claude-restore",
            "autostart",
            "update",
          ].includes(feedback.key)
            ? feedback
            : null
        }
      />
    </Flexbox>
  );
}

function ActivityView({
  logs,
  onClear,
  onCopy,
}: {
  logs: LogLine[];
  onClear: () => void;
  onCopy: () => void;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    ref.current?.scrollTo({ top: ref.current.scrollHeight });
  }, [logs]);

  return (
    <Block
      variant="outlined"
      padding={0}
      style={{ height: "100%", display: "flex", flexDirection: "column", minHeight: 0 }}
    >
      <Flexbox style={{ height: "100%", minHeight: 0 }}>
        <Flexbox
          horizontal
          distribution="space-between"
          align="center"
          padding={12}
          style={{ borderBottom: "1px solid rgba(255,255,255,0.06)", flexShrink: 0 }}
        >
          <Text weight={600}>事件流</Text>
          <Flexbox horizontal gap={8}>
            <Button size="small" icon={Copy} onClick={onCopy}>
              复制
            </Button>
            <Button size="small" onClick={onClear}>
              清空
            </Button>
          </Flexbox>
        </Flexbox>
        <div className="mini-log" ref={ref} style={{ flex: 1 }}>
          {logs.length === 0 ? (
            <Text type="secondary">暂无输出 · 启动/停止网关时会推送事件</Text>
          ) : (
            logs.map((l) => (
              <div className="log-line" key={l.id}>
                <span style={{ opacity: 0.7, fontWeight: 700 }}>{l.level}</span>
                <span>{l.message}</span>
              </div>
            ))
          )}
        </div>
      </Flexbox>
    </Block>
  );
}

function CreditBar({ info }: { info: ProjectInfo | null }) {
  return (
    <div className="credit-bar">
      <Icon icon={ExternalLink} size="small" />
      <a
        href={info?.github || GITHUB}
        onClick={(e) => {
          e.preventDefault();
          void openUrl(info?.github || GITHUB);
        }}
      >
        codex-chat-gateway
      </a>
      <span>·</span>
      <span>@{info?.credits.owner || "xuyuanzhang1122"}</span>
      <span>·</span>
      <a
        href={LOBE_UI}
        onClick={(e) => {
          e.preventDefault();
          void openUrl(LOBE_UI);
        }}
      >
        LobeHub
      </a>
      <span>·</span>
      <span>MIT · 非 OpenAI 官方</span>
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
  const [modelId, setModelId] = useState(profile?.model_id ?? "");
  const [routingEnabled, setRoutingEnabled] = useState(profile?.routing_enabled ?? true);
  const [routingWeight, setRoutingWeight] = useState(profile?.routing_weight ?? 1);
  const [msg, setMsg] = useState<string>("");
  const [msgType, setMsgType] = useState<"success" | "error" | "info">("info");
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
      setMsgType("error");
      setMsg("请填写有效的 HTTP(S) 地址");
      return;
    }
    if (!key.trim()) {
      setMsgType("error");
      setMsg("请填写 API Key");
      return;
    }
    setFetching(true);
    setMsgType("info");
    setMsg("正在拉取模型列表…");
    try {
      const ids = await api.fetchModels(base, key.trim());
      setPicker(ids);
      setMsgType("success");
      setMsg(`共 ${ids.length} 个模型`);
    } catch (e) {
      setMsgType("error");
      setMsg(`${String(e)} · 可手动填写模型 ID`);
    } finally {
      setFetching(false);
    }
  };

  return (
    <Modal
      open
      title={mode === "add" ? "添加上游模型" : "编辑模型"}
      onCancel={onClose}
      footer={
        picker ? (
          <Button onClick={() => setPicker(null)}>返回</Button>
        ) : (
          <Flexbox horizontal gap={8} distribution="flex-end">
            <Button onClick={onClose} disabled={saving}>
              取消
            </Button>
            <Button
              type="primary"
              loading={saving}
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
                        routing_enabled: routingEnabled,
                        routing_weight: Math.max(1, Math.min(100, routingWeight || 1)),
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
            </Button>
          </Flexbox>
        )
      }
      width={480}
    >
      {picker ? (
        <Flexbox gap={12}>
          <SearchBar
            value={filter}
            onInputChange={(v) => setFilter(v)}
            placeholder="搜索模型 ID"
            allowClear
          />
          <Flexbox gap={4} style={{ maxHeight: 320, overflow: "auto" }}>
            {filtered.map((id) => (
              <Block
                key={id}
                clickable
                variant={modelId === id ? "filled" : "outlined"}
                padding={10}
                onClick={() => {
                  setModelId(id);
                  if (!name.trim()) setName(id);
                  setPicker(null);
                  setMsgType("success");
                  setMsg(`已选择 ${id}`);
                }}
              >
                <Text fontSize={12}>{id}</Text>
              </Block>
            ))}
            {filtered.length === 0 && <Empty title="无匹配项" />}
          </Flexbox>
        </Flexbox>
      ) : (
        <Form gap={12} layout="vertical">
          <FormItem label="名称">
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="配置名称" />
          </FormItem>
          <FormItem label="API Base URL">
            <Input
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://api.example.com/v1"
            />
          </FormItem>
          <FormItem label="API Key">
            <InputPassword
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder="sk-…"
            />
          </FormItem>
          <FormItem label="模型 ID">
            <Flexbox horizontal gap={8}>
              <Input
                value={modelId}
                onChange={(e) => setModelId(e.target.value)}
                placeholder="deepseek-chat"
                style={{ flex: 1 }}
              />
              <Button loading={fetching} onClick={() => void fetchList()}>
                在线获取
              </Button>
            </Flexbox>
          </FormItem>
          <FormItem label="同模型分流">
            <Flexbox horizontal distribution="space-between" align="center" gap={12}>
              <Flexbox gap={2}>
                <Text fontSize={12}>作为该模型的可选分流上游</Text>
                <Text type="secondary" fontSize={11}>
                  同一会话会优先保持在同一家，故障时才切换。
                </Text>
              </Flexbox>
              <Switch checked={routingEnabled} onChange={setRoutingEnabled} />
            </Flexbox>
          </FormItem>
          <FormItem label="分流权重">
            <Flexbox horizontal gap={10} align="center">
              <InputNumber
                min={1}
                max={100}
                precision={0}
                value={routingWeight}
                disabled={!routingEnabled}
                onChange={(value) => setRoutingWeight(value ?? 1)}
              />
              <Text type="secondary" fontSize={11}>
                例如 3:1 表示新会话约 75%:25%；不会拆分已建立的会话。
              </Text>
            </Flexbox>
          </FormItem>
          {msg && (
            <Alert
              type={msgType === "success" ? "success" : msgType === "error" ? "error" : "info"}
              showIcon
              message={msg}
            />
          )}
        </Form>
      )}
    </Modal>
  );
}

export default App;
