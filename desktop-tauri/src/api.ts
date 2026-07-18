import { invoke } from "@tauri-apps/api/core";
import type {
  ActionResult,
  GatewayStatus,
  ModelInput,
  ModelStore,
  ProjectInfo,
} from "./types";

export const api = {
  status: () => invoke<GatewayStatus>("get_status"),
  projectInfo: () => invoke<ProjectInfo>("get_project_info"),
  listModels: () => invoke<ModelStore>("list_models"),
  createModel: (input: ModelInput) => invoke<ModelStore>("create_model", { input }),
  editModel: (id: string, input: ModelInput) =>
    invoke<ModelStore>("edit_model", { id, input }),
  removeModel: (id: string) => invoke<ModelStore>("remove_model", { id }),
  makeDefault: (id: string) => invoke<ModelStore>("make_default", { id }),
  fetchModels: (baseUrl: string, apiKey: string) =>
    invoke<string[]>("fetch_models", { baseUrl, apiKey }),
  /** Fire-and-forget — progress via gateway:// events */
  start: () => invoke<void>("gateway_start"),
  stop: () => invoke<void>("gateway_stop"),
  restart: () => invoke<void>("gateway_restart"),
  check: () => invoke<ActionResult>("gateway_check"),
  logsDir: () => invoke<string>("get_logs_dir"),
  toggleAutostart: (enable: boolean) => invoke<string>("toggle_autostart", { enable }),
  runScript: (name: string) => invoke<ActionResult>("run_script", { name }),
};
