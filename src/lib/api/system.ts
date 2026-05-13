import { invoke } from "@tauri-apps/api/core";

export const getAutostart = () =>
  invoke<boolean>("get_autostart_enabled");

export const setAutostart = (enabled: boolean) =>
  invoke<void>("set_autostart_enabled", { enabled });

export const backup = () =>
  invoke<string>("backup_database");

export const factoryReset = () =>
  invoke<void>("factory_reset");
