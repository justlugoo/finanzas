import { invoke } from "@tauri-apps/api/core";
import type { MetaV2 } from "$lib/types";

export const list = () =>
  invoke<MetaV2[]>("metas_list_v2");
