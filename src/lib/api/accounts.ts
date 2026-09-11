import { invoke } from "@tauri-apps/api/core";
import type { Account } from "$lib/types";

export const list = () =>
  invoke<Account[]>("account_list");
