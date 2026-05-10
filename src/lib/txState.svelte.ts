export const txState = $state({ version: 0 });
export function bumpTxVersion() { txState.version++; }
