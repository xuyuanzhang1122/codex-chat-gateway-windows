import { memo, useMemo } from "react";
import {
  Background,
  BackgroundVariant,
  BaseEdge,
  Controls,
  Handle,
  MarkerType,
  Position,
  ReactFlow,
  getBezierPath,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeProps,
  type NodeTypes,
} from "@xyflow/react";
import { Activity, Cpu, Globe2, Radio, Zap } from "lucide-react";
import type {
  GatewayStatus,
  ModelProfile,
  ModelStore,
  RoutingTrafficRoute,
  RoutingTrafficStore,
} from "./types";
import "@xyflow/react/dist/style.css";
import "./routing-map.css";

type ModelNodeData = {
  label: string;
  enabled: boolean;
  routes: number;
  hits: number;
};

type UpstreamNodeData = {
  label: string;
  host: string;
  enabled: boolean;
  historical: boolean;
  hits: number;
};

type ModelFlowNode = Node<ModelNodeData, "model">;
type UpstreamFlowNode = Node<UpstreamNodeData, "upstream">;
type ElectricFlowEdge = Edge<{ fresh: boolean; hits: number }, "electric">;

const normalizeModel = (value: string) => {
  const normalized = value.trim().toLowerCase();
  const [provider, ...rest] = normalized.split("/");
  return ["openai", "custom_openai", "deepseek"].includes(provider) && rest.length > 0
    ? rest.join("/")
    : normalized;
};

const safeHost = (baseUrl: string) => {
  try {
    return new URL(baseUrl).hostname || baseUrl;
  } catch {
    return baseUrl.replace(/^https?:\/\//i, "").split("/")[0] || "unknown-upstream";
  }
};

const modelRoutingEnabled = (store: ModelStore, modelId: string) => {
  const rules = store.routing.model_rules ?? [];
  const rule = rules.find((item) => normalizeModel(item.model_id) === normalizeModel(modelId));
  if (rules.length > 0) return !!rule?.enabled;
  const current = store.profiles.find((item) => item.id === store.default_id);
  return !!store.routing.enabled && normalizeModel(current?.model_id ?? "") === normalizeModel(modelId);
};

const ModelNode = memo(function ModelNode({ data }: NodeProps<ModelFlowNode>) {
  return (
    <div className={`flow-model-node${data.enabled ? " is-enabled" : ""}`}>
      <div className="flow-node-kicker">
        <Cpu size={12} /> MODEL CHANNEL
      </div>
      <div className="flow-model-name">{data.label}</div>
      <div className="flow-node-meta">
        <span>{data.routes} 条已建立线路</span>
        <span>{data.hits} 次命中</span>
      </div>
      <Handle type="source" position={Position.Bottom} className="flow-handle" />
    </div>
  );
});

const UpstreamNode = memo(function UpstreamNode({ data }: NodeProps<UpstreamFlowNode>) {
  return (
    <div
      className={`flow-upstream-node${data.hits > 0 ? " has-traffic" : ""}${
        data.enabled ? " is-enabled" : ""
      }`}
    >
      <Handle type="target" position={Position.Top} className="flow-handle" />
      <div className="flow-site-icon">
        <Globe2 size={17} />
      </div>
      <div className="flow-site-copy">
        <div className="flow-site-name">{data.label}</div>
        <div className="flow-site-host">{data.host}</div>
      </div>
      <div className="flow-site-state">
        <span className="flow-site-count">{data.hits}</span>
        <span>{data.historical ? "历史" : data.enabled ? "READY" : "OFF"}</span>
      </div>
    </div>
  );
});

const ElectricEdge = memo(function ElectricEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerEnd,
  data,
}: EdgeProps<ElectricFlowEdge>) {
  const [edgePath] = getBezierPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
    curvature: 0.32,
  });

  return (
    <g className={`electric-route${data?.fresh ? " is-fresh" : ""}`} data-edge-id={id}>
      <BaseEdge path={edgePath} className="electric-route-halo" />
      <BaseEdge path={edgePath} markerEnd={markerEnd} className="electric-route-core" />
      <path d={edgePath} className="electric-route-dashes" />
      <circle r="3.3" className="electric-packet electric-packet-a">
        <animateMotion dur="1.75s" repeatCount="indefinite" path={edgePath} />
      </circle>
      <circle r="2.2" className="electric-packet electric-packet-b">
        <animateMotion dur="1.75s" begin="-0.88s" repeatCount="indefinite" path={edgePath} />
      </circle>
    </g>
  );
});

const nodeTypes: NodeTypes = { model: ModelNode, upstream: UpstreamNode };
const edgeTypes = { electric: ElectricEdge };

type ModelGroup = {
  key: string;
  label: string;
  profiles: Array<{ profile: ModelProfile | null; route: RoutingTrafficRoute | null }>;
};

function buildGraph(store: ModelStore, traffic: RoutingTrafficStore) {
  const currentProfiles = new Map(store.profiles.map((profile) => [profile.id, profile]));
  const routeByProfile = new Map(traffic.routes.map((route) => [route.profile_id, route]));
  const groups = new Map<string, ModelGroup>();

  const ensureGroup = (modelId: string) => {
    const key = normalizeModel(modelId);
    let group = groups.get(key);
    if (!group) {
      group = { key, label: modelId, profiles: [] };
      groups.set(key, group);
    }
    return group;
  };

  for (const profile of store.profiles) {
    ensureGroup(profile.model_id).profiles.push({
      profile,
      route: routeByProfile.get(profile.id) ?? null,
    });
  }
  for (const route of traffic.routes) {
    if (currentProfiles.has(route.profile_id)) continue;
    ensureGroup(route.model_id).profiles.push({ profile: null, route });
  }

  const nodes: Array<ModelFlowNode | UpstreamFlowNode> = [];
  const edges: ElectricFlowEdge[] = [];
  let cursorX = 40;
  const now = Date.now();

  for (const group of groups.values()) {
    const groupWidth = Math.max(300, group.profiles.length * 260);
    const modelNodeId = `model:${group.key}`;
    const routes = traffic.routes.filter((route) => normalizeModel(route.model_id) === group.key);
    nodes.push({
      id: modelNodeId,
      type: "model",
      position: { x: cursorX + groupWidth / 2 - 130, y: 28 },
      data: {
        label: group.label,
        enabled: modelRoutingEnabled(store, group.label),
        routes: routes.length,
        hits: routes.reduce((total, route) => total + route.hit_count, 0),
      },
      draggable: false,
      selectable: false,
    });

    group.profiles.forEach(({ profile, route }, index) => {
      const profileId = profile?.id ?? route!.profile_id;
      const upstreamId = `upstream:${profileId}`;
      nodes.push({
        id: upstreamId,
        type: "upstream",
        position: { x: cursorX + index * 260 + 25, y: 330 + (index % 2) * 42 },
        data: {
          label: profile?.name ?? route!.profile_name,
          host: profile ? safeHost(profile.base_url) : route!.upstream_host,
          enabled: profile?.routing_enabled ?? false,
          historical: !profile,
          hits: route?.hit_count ?? 0,
        },
        draggable: false,
        selectable: false,
      });

      if (route) {
        const seen = Date.parse(route.last_seen_at);
        edges.push({
          id: `route:${group.key}:${profileId}`,
          type: "electric",
          source: modelNodeId,
          target: upstreamId,
          data: {
            fresh: Number.isFinite(seen) && now - seen < 4_500,
            hits: route.hit_count,
          },
          markerEnd: {
            type: MarkerType.ArrowClosed,
            color: "#b9ff3d",
            width: 16,
            height: 16,
          },
          zIndex: 3,
        });
      }
    });
    cursorX += groupWidth + 120;
  }

  return { nodes, edges };
}

export function RoutingMapView({
  store,
  traffic,
  status,
  error,
}: {
  store: ModelStore;
  traffic: RoutingTrafficStore;
  status: GatewayStatus;
  error: string | null;
}) {
  const graph = useMemo(() => buildGraph(store, traffic), [store, traffic]);
  const totalHits = traffic.routes.reduce((total, route) => total + route.hit_count, 0);
  const latest = traffic.routes
    .map((route) => Date.parse(route.last_seen_at))
    .filter(Number.isFinite)
    .sort((a, b) => b - a)[0];

  return (
    <section className="routing-map-page">
      <div className="routing-map-toolbar">
        <div className="routing-map-live">
          <span className={`routing-map-live-dot${status.running ? " is-live" : ""}`} />
          <div>
            <strong>{status.running ? "正在监听真实流量" : "等待网关启动"}</strong>
            <span>连接一旦捕获即常驻 · 不记录提示词与响应正文</span>
          </div>
        </div>
        <div className="routing-map-metrics">
          <div><Zap size={13} /><strong>{traffic.routes.length}</strong><span>线路</span></div>
          <div><Activity size={13} /><strong>{totalHits}</strong><span>命中</span></div>
          <div><Radio size={13} /><strong>{latest ? new Date(latest).toLocaleTimeString("zh-CN", { hour12: false }) : "--:--"}</strong><span>最近</span></div>
        </div>
      </div>

      <div className="routing-map-canvas">
        {error && <div className="routing-map-error">轨迹读取异常 · {error}</div>}
        {graph.nodes.length === 0 ? (
          <div className="routing-map-empty">
            <div className="routing-map-empty-orbit"><Zap size={24} /></div>
            <strong>尚未配置可视化节点</strong>
            <span>先在“模型”页添加上游并开启分流。</span>
          </div>
        ) : (
          <ReactFlow
            nodes={graph.nodes}
            edges={graph.edges}
            nodeTypes={nodeTypes}
            edgeTypes={edgeTypes}
            fitView
            fitViewOptions={{ padding: 0.16, maxZoom: 1.2 }}
            minZoom={0.35}
            maxZoom={1.65}
            nodesConnectable={false}
            nodesDraggable={false}
            elementsSelectable={false}
            panOnDrag
            zoomOnDoubleClick={false}
            proOptions={{ hideAttribution: false }}
          >
            <Background variant={BackgroundVariant.Dots} gap={22} size={1} color="rgba(181,255,48,.13)" />
            <Controls showInteractive={false} position="bottom-right" />
          </ReactFlow>
        )}
        <div className="routing-map-axis axis-models">MODELS</div>
        <div className="routing-map-axis axis-upstreams">UPSTREAM SITES</div>
      </div>
    </section>
  );
}
