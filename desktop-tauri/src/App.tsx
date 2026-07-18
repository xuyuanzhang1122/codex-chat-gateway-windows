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
  Snippet,
  Tag,
  Text,
  Tooltip,
} from "@lobehub/ui";
import {
  Activity,
  Bot,
  CheckCircle2,
  Copy,
  ExternalLink,
  FolderOpen,
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
} from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { ask, message as dialogMessage } from "@tauri-apps/plugin-dialog";
import { api } from "./api";
import { TitleBar } from "./TitleBar";
import type {
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
};

const GITHUB = "https://github.com/xuyuanzhang1122/codex-chat-gateway-windows";
const LOBE_UI = "https://ui.lobehub.com";

function App() {
  const [splash, setSplash] = useState(true);
  const [splashExit, setSplashExit] = useState(false);
  const [page, setPage] = useState<Page>("gateway");
  const [status, setStatus] = useState<GatewayStatus>(emptyStatus);
  const [store, setStore] = useState<ModelStore>({ version: 1, default_id: "", profiles: [] });
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [logs, setLogs] = useState<LogLine[]>([]);
  const logId = useRef(0);
  const [dialog, setDialog] = useState<null | { mode: "add" | "edit"; profile?: ModelProfile }>(
    null,
  );
  const ready = !splash;

  const pushLog = useCallback((level: LogLevel, msg: string) => {
    if (!msg) return;
    setLogs((prev) => {
      const next = [...prev, { id: ++logId.current, level, message: msg }];
      return next.length > 250 ? next.slice(-250) : next;
    });
  }, []);

  // Event-driven backend (no heavy polling loop)
  useEffect(() => {
    if (!ready) return;
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
        pushLog("INFO", "Studio 已就绪（事件驱动后端）");
        pushLog("DIM", proj.root);
      } catch (e) {
        pushLog("ERR", `初始化失败: ${String(e)}`);
      }

      const u1 = await listen<GatewayStatus>("gateway://status", (e) => {
        setStatus(e.payload);
      });
      const u2 = await listen<{ level: string; message: string }>("gateway://log", (e) => {
        const lv = (e.payload.level || "DIM").toUpperCase() as LogLevel;
        pushLog(
          lv === "OK" || lv === "ERR" || lv === "INFO" || lv === "DIM" ? lv : "DIM",
          e.payload.message,
        );
      });
      const u3 = await listen<{ ok: boolean; message: string }>("gateway://action", (e) => {
        // status already streamed; keep models warm after start
        if (e.payload.ok) {
          void api.listModels().then(setStore).catch(() => undefined);
          void api.projectInfo().then(setInfo).catch(() => undefined);
        }
      });
      unsubs = [u1, u2, u3];
    })();

    return () => {
      for (const u of unsubs) u();
    };
  }, [ready, pushLog]);

  const enter = () => {
    setSplashExit(true);
    window.setTimeout(() => setSplash(false), 380);
  };

  const hasDefault = store.profiles.some((p) => p.id === store.default_id);
  const busy = status.busy;
  const live = status.running || status.healthy;

  const onStart = () => {
    if (!hasDefault) {
      pushLog("DIM", "请先添加默认模型");
      setPage("models");
      setDialog({ mode: "add" });
      return;
    }
    void api.start();
  };

  const saveModel = async (input: ModelInput, editId?: string) => {
    const next = editId
      ? await api.editModel(editId, input)
      : await api.createModel(input);
    setStore(next);
    setDialog(null);
    pushLog("OK", editId ? `已更新 ${input.name}` : `已保存 ${input.name}`);
    const touched =
      (editId && editId === store.default_id) || (!editId && next.profiles.length === 1);
    if (live && touched) {
      const yes = await ask("配置已变更，是否立即重启网关？", {
        title: "重启网关",
        kind: "info",
        okLabel: "重启",
        cancelLabel: "稍后",
      });
      if (yes) void api.restart();
    }
  };

  const navItems = useMemo(
    () => [
      { key: "gateway" as const, title: "网关", icon: Server },
      { key: "models" as const, title: "模型", icon: Layers3 },
      { key: "clients" as const, title: "客户端", icon: Users },
      { key: "activity" as const, title: "日志", icon: Activity },
    ],
    [],
  );

  return (
    <>
      {splash && (
        <div className={`splash ${splashExit ? "exit" : ""}`}>
          <Flexbox align="center" gap={16}>
            <img src="/gateway-logo.png" width={88} height={88} alt="" />
            <Text as="h1" fontSize={28} weight={700}>
              Codex Chat Gateway
            </Text>
            <Text type="secondary" fontSize={12} style={{ letterSpacing: "0.2em" }}>
              STUDIO CONSOLE
            </Text>
            <Button type="primary" size="large" glass shadow onClick={enter}>
              进入控制台
            </Button>
            <Text type="secondary" fontSize={12}>
              本地模型桥 · 密钥不出本机
            </Text>
          </Flexbox>
        </div>
      )}

      <div className="app-shell">
        <TitleBar />
        <div className="workspace">
          <SideNav
            avatar={<img src="/gateway-logo.png" width={32} height={32} alt="" style={{ borderRadius: 8 }} />}
            bottomActions={
              <Tooltip title="GitHub 仓库">
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
            <Flexbox gap={16} style={{ minHeight: "100%" }}>
              <PageHeader page={page} version={info?.version} status={status} />

              <div className="page-pane" style={{ flex: 1 }}>
                {page === "gateway" && (
                  <GatewayView
                    status={status}
                    store={store}
                    busy={busy}
                    live={live}
                    onStart={onStart}
                    onStop={() => void api.stop()}
                    onRestart={() => {
                      if (!hasDefault) {
                        setPage("models");
                        return;
                      }
                      void api.restart();
                    }}
                    onCheck={() => void api.check()}
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
                  <ModelsView
                    store={store}
                    selectedId={selectedId}
                    busy={busy}
                    onSelect={setSelectedId}
                    onAdd={() => setDialog({ mode: "add" })}
                    onEdit={() => {
                      const p = store.profiles.find((x) => x.id === selectedId);
                      if (!p) {
                        pushLog("DIM", "请先选择模型");
                        return;
                      }
                      setDialog({ mode: "edit", profile: p });
                    }}
                    onDefault={async () => {
                      if (!selectedId) return;
                      try {
                        const next = await api.makeDefault(selectedId);
                        setStore(next);
                        pushLog("OK", "已设为默认");
                        if (live) {
                          const yes = await ask("是否立即重启网关？", {
                            title: "重启",
                            kind: "info",
                            okLabel: "重启",
                            cancelLabel: "稍后",
                          });
                          if (yes) void api.restart();
                        }
                      } catch (e) {
                        pushLog("ERR", String(e));
                      }
                    }}
                    onDelete={async () => {
                      if (!selectedId) return;
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
                        setStore(await api.removeModel(selectedId));
                        setSelectedId(null);
                        pushLog("OK", `已删除 ${p.name}`);
                      } catch (e) {
                        pushLog("ERR", String(e));
                      }
                    }}
                    onRefresh={() =>
                      void api
                        .listModels()
                        .then(setStore)
                        .then(() => pushLog("OK", "模型列表已刷新"))
                    }
                  />
                )}

                {page === "clients" && (
                  <ClientsView
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
                        pushLog("INFO", `▶ ${label}`);
                        try {
                          await api.runScript(script);
                          setInfo(await api.projectInfo());
                        } catch (e) {
                          pushLog("ERR", String(e));
                        }
                      })();
                    }}
                    onAutostart={() => {
                      const enable = !info?.autostart;
                      void (async () => {
                        try {
                          const msg = await api.toggleAutostart(enable);
                          pushLog("OK", msg);
                          setInfo(await api.projectInfo());
                        } catch (e) {
                          pushLog("ERR", String(e));
                        }
                      })();
                    }}
                  />
                )}

                {page === "activity" && (
                  <ActivityView logs={logs} onClear={() => setLogs([])} onCopy={() => {
                    const text = logs.map((l) => `${l.level}  ${l.message}`).join("\n");
                    void navigator.clipboard.writeText(text).then(
                      () => pushLog("OK", "已复制日志"),
                      () => pushLog("ERR", "复制失败"),
                    );
                  }} />
                )}
              </div>

              <CreditBar info={info} />
            </Flexbox>
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
              await dialogMessage(String(e), { title: "保存失败", kind: "error" });
              throw e;
            }
          }}
        />
      )}
    </>
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

function GatewayView({
  status,
  store,
  busy,
  live,
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

  return (
    <Flexbox gap={14}>
      {!status.is_our_gateway && status.healthy && (
        <Alert
          type="warning"
          showIcon
          message="端口 4000 有响应，但可能不是本网关（缺少 codex-chat 路由）"
        />
      )}

      <Flexbox horizontal gap={12} style={{ flexWrap: "wrap" }}>
        <Block variant="outlined" padding={16} style={{ flex: "1 1 200px", minWidth: 180 }}>
          <Text type="secondary" fontSize={12}>
            状态
          </Text>
          <Text fontSize={22} weight={700} style={{ marginTop: 8 }}>
            {live ? (status.is_our_gateway ? "运行中" : "端口占用") : "已停止"}
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

      <Block variant="outlined" padding={16}>
        <Flexbox gap={14}>
          <Flexbox horizontal gap={10} align="center" style={{ flexWrap: "wrap" }}>
            <Text type="secondary" fontSize={12}>
              Endpoint
            </Text>
            <Snippet>{status.endpoint}</Snippet>
          </Flexbox>
          <Flexbox horizontal gap={8} style={{ flexWrap: "wrap" }}>
            <Button
              type="primary"
              icon={Play}
              loading={status.phase === "starting"}
              disabled={busy || (live && status.is_our_gateway)}
              onClick={onStart}
            >
              启动
            </Button>
            <Button
              icon={Square}
              loading={status.phase === "stopping"}
              disabled={busy || !live}
              onClick={onStop}
            >
              停止
            </Button>
            <Button icon={RotateCcw} disabled={busy || !live} onClick={onRestart}>
              重启
            </Button>
            <Button icon={CheckCircle2} disabled={busy} onClick={onCheck}>
              健康检查
            </Button>
            <Button icon={FolderOpen} variant="outlined" onClick={onLogs}>
              日志目录
            </Button>
            <Button icon={Bot} variant="outlined" onClick={onUi}>
              LiteLLM UI
            </Button>
          </Flexbox>
        </Flexbox>
      </Block>
    </Flexbox>
  );
}

function ModelsView({
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
    <Flexbox gap={14}>
      <Flexbox horizontal distribution="space-between" align="center">
        <Text type="secondary">{store.profiles.length} 个配置</Text>
        <Flexbox horizontal gap={8}>
          <Button type="primary" icon={Plus} disabled={busy} onClick={onAdd}>
            添加模型
          </Button>
          <Button icon={RefreshCw} disabled={busy} onClick={onRefresh}>
            刷新
          </Button>
        </Flexbox>
      </Flexbox>

      {store.profiles.length === 0 ? (
        <Empty
          title="还没有上游模型"
          description="添加 DeepSeek / Kimi 等 Chat Completions 接口"
          action={
            <Button type="primary" icon={Plus} onClick={onAdd}>
              立即添加
            </Button>
          }
        />
      ) : (
        <>
          <Flexbox gap={10} horizontal style={{ flexWrap: "wrap" }}>
            {store.profiles.map((p) => {
              const isDefault = p.id === store.default_id;
              const selected = selectedId === p.id;
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
                  </Flexbox>
                </Block>
              );
            })}
          </Flexbox>
          <Flexbox horizontal gap={8}>
            <Button icon={Star} disabled={busy} onClick={onDefault}>
              设为默认
            </Button>
            <Button icon={Pencil} disabled={busy} onClick={onEdit}>
              编辑
            </Button>
            <Button danger icon={Trash2} disabled={busy} onClick={onDelete}>
              删除
            </Button>
          </Flexbox>
        </>
      )}
    </Flexbox>
  );
}

function ClientsView({
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
    <Flexbox gap={12} horizontal style={{ flexWrap: "wrap" }}>
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
              disabled={busy}
              onClick={() => onScript("配置 Codex", "configure-codex.ps1")}
            >
              配置
            </Button>
            <Button
              danger
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
              disabled={busy}
              onClick={() => onScript("配置 Claude Desktop", "configure-claude-desktop.ps1")}
            >
              配置
            </Button>
            <Button
              danger
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
            </Button>
          </Flexbox>
        </Flexbox>
      </Block>

      <Block variant="outlined" padding={18} style={{ flex: "1 1 100%" }}>
        <Flexbox horizontal distribution="space-between" align="center" gap={12}>
          <Flexbox gap={4}>
            <Text weight={700} fontSize={16}>
              登录自启
            </Text>
            <Text type="secondary" fontSize={13}>
              登录 Windows 后自动启动网关进程（不启动本控制台）
            </Text>
          </Flexbox>
          <Button icon={Settings2} disabled={busy} onClick={onAutostart}>
            {autostart ? "已开启 · 点击关闭" : "已关闭 · 点击开启"}
          </Button>
        </Flexbox>
      </Block>
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
    <Block variant="outlined" padding={0}>
      <Flexbox>
        <Flexbox
          horizontal
          distribution="space-between"
          align="center"
          padding={12}
          style={{ borderBottom: "1px solid rgba(255,255,255,0.06)" }}
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
        <div className="log-scroll" ref={ref}>
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
        github.com/xuyuanzhang1122/codex-chat-gateway-windows
      </a>
      <span>·</span>
      <span>Owner: xuyuanzhang1122</span>
      <span>·</span>
      <span>UI by</span>
      <a
        href={LOBE_UI}
        onClick={(e) => {
          e.preventDefault();
          void openUrl(LOBE_UI);
        }}
      >
        LobeHub UI
      </a>
      <span>·</span>
      <span>MIT · 社区网关，非 OpenAI 官方</span>
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
