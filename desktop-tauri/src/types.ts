export interface ModelProfile {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  model_id: string;
  litellm_model: string;
  routing_enabled: boolean;
  routing_weight: number;
}

export interface RoutingSettings {
  enabled: boolean;
  affinity_ttl_seconds: number;
  model_rules: ModelRoutingRule[];
}

export interface ModelRoutingRule {
  model_id: string;
  enabled: boolean;
}

export interface ModelStore {
  version: number;
  default_id: string;
  profiles: ModelProfile[];
  routing: RoutingSettings;
}

export interface ModelInput {
  name: string;
  base_url: string;
  api_key: string;
  model_id: string;
  routing_enabled: boolean;
  routing_weight: number;
}

export interface RoutingTrafficRoute {
  model_id: string;
  profile_id: string;
  profile_name: string;
  upstream_host: string;
  hit_count: number;
  first_seen_at: string;
  last_seen_at: string;
}

export interface RoutingTrafficStore {
  version: number;
  routes: RoutingTrafficRoute[];
}

/** Parsed plaintext api.txt-style config from the backend. */
export interface ParsedApiText {
  base_url: string;
  api_key: string;
  models: string[];
  name_hint: string | null;
  model_missing: boolean;
}

export type GatewayPhase = "stopped" | "starting" | "running" | "stopping" | "error";

export interface GatewayStatus {
  phase: GatewayPhase;
  running: boolean;
  healthy: boolean;
  is_our_gateway: boolean;
  endpoint: string;
  pid: number | null;
  model: string | null;
  started_at: string | null;
  uptime: string | null;
  default_model_name: string | null;
  message: string;
  routes: string[];
  busy: boolean;
}

export interface ActionResult {
  ok: boolean;
  message: string;
  logs: string[];
  status: GatewayStatus;
}

export interface ProjectInfo {
  root: string;
  version: string;
  endpoint: string;
  autostart: boolean;
  github: string;
  credits: {
    project: string;
    repository: string;
    owner: string;
    ui_kit: string;
    ui_kit_name: string;
  };
}

export type LogLevel = "INFO" | "OK" | "ERR" | "DIM";

export interface LogLine {
  id: number;
  level: LogLevel;
  message: string;
}
