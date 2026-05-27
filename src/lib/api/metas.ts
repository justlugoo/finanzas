import { invoke } from "@tauri-apps/api/core";
import type { Meta } from "$lib/types";

export const list = () =>
  invoke<Meta[]>("metas_list");
