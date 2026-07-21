import { invoke } from "@tauri-apps/api/core";
import type {
  ActionResult,
  GatewayStatus,
  ModelInput,
  ModelStore,
  ParsedApiText,
  ProjectInfo,
  RoutingTrafficStore,
} from "./types";

export const api = {
  status: () => invoke<GatewayStatus>("get_status"),
  projectInfo: () => invoke<ProjectInfo>("get_project_info"),
  routingTraffic: () => invoke<RoutingTrafficStore>("get_routing_traffic"),
  listModels: () => invoke<ModelStore>("list_models"),
  createModel: (input: ModelInput) => invoke<ModelStore>("create_model", { input }),
  editModel: (id: string, input: ModelInput) =>
    invoke<ModelStore>("edit_model", { id, input }),
  removeModel: (id: string) => invoke<ModelStore>("remove_model", { id }),
  makeDefault: (id: string) => invoke<ModelStore>("make_default", { id }),
  configureModelRouting: (modelId: string, enabled: boolean) =>
    invoke<ModelStore>("configure_model_routing", { modelId, enabled }),
  configureProfileRouting: (id: string, enabled: boolean) =>
    invoke<ModelStore>("configure_profile_routing", { id, enabled }),
  fetchModels: (baseUrl: string, apiKey: string) =>
    invoke<string[]>("fetch_models", { baseUrl, apiKey }),
  parseModelText: (text: string) => invoke<ParsedApiText>("parse_model_text", { text }),
  parseModelFile: (path: string) => invoke<ParsedApiText>("parse_model_file", { path }),
  importModelProfiles: (
    baseUrl: string,
    apiKey: string,
    modelIds: string[],
    nameHint?: string | null,
  ) =>
    invoke<ModelStore>("import_model_profiles", {
      baseUrl,
      apiKey,
      modelIds,
      nameHint: nameHint ?? null,
    }),
  /** Fire-and-forget — progress via gateway:// events */
  start: () => invoke<void>("gateway_start"),
  stop: () => invoke<void>("gateway_stop"),
  restart: () => invoke<void>("gateway_restart"),
  reloadConfig: () => invoke<ActionResult>("gateway_reload_config"),
  check: () => invoke<ActionResult>("gateway_check"),
  logsDir: () => invoke<string>("get_logs_dir"),
  openLogsDir: () => invoke<string>("open_logs_dir"),
  toggleAutostart: (enable: boolean) => invoke<string>("toggle_autostart", { enable }),
  runScript: (name: string) => invoke<ActionResult>("run_script", { name }),
};
