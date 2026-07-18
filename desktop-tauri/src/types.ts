export interface ModelProfile {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  model_id: string;
  litellm_model: string;
}

export interface ModelStore {
  version: number;
  default_id: string;
  profiles: ModelProfile[];
}

export interface ModelInput {
  name: string;
  base_url: string;
  api_key: string;
  model_id: string;
}

export interface GatewayStatus {
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
}

export type LogLevel = "INFO" | "OK" | "ERR" | "DIM";

export interface LogLine {
  id: number;
  level: LogLevel;
  message: string;
}
