import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { GuidancePayload, OrchestratorEvent } from "./types";

export const COMMAND = {
  startSession: "start_session",
  confirmResponse: "confirm_response",
  cancelSession: "cancel_session",
  requestStuckHelp: "request_stuck_help",
  showOverlayAnchor: "show_overlay_anchor",
  clearOverlay: "clear_overlay",
  togglePanel: "toggle_panel",
  panelVisible: "panel_visible",
} as const;

export const EVENT = {
  guidance: "roota://guidance",
  anchor: "roota://anchor",
  clearAnchor: "roota://anchor-clear",
  panelVisible: "roota://panel-visible",
} as const;

export function newOrchestratorChannel(): Channel<OrchestratorEvent> {
  return new Channel<OrchestratorEvent>();
}

export async function startSession(
  utterance: string,
  channel: Channel<OrchestratorEvent>,
): Promise<void> {
  await invoke(COMMAND.startSession, { utterance, onEvent: channel });
}

export async function confirmResponse(accepted: boolean): Promise<void> {
  await invoke(COMMAND.confirmResponse, { accepted });
}

export async function cancelSession(): Promise<void> {
  await invoke(COMMAND.cancelSession);
}

export async function requestStuckHelp(): Promise<void> {
  await invoke(COMMAND.requestStuckHelp);
}

export async function togglePanel(): Promise<boolean> {
  return invoke<boolean>(COMMAND.togglePanel);
}

export async function isPanelVisible(): Promise<boolean> {
  return invoke<boolean>(COMMAND.panelVisible);
}

export async function listenGuidance(
  handler: (payload: GuidancePayload) => void,
): Promise<UnlistenFn> {
  return listen<GuidancePayload>(EVENT.guidance, (e) => handler(e.payload));
}

export async function listenAnchor(
  handler: (payload: { x: number; y: number; label: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ x: number; y: number; label: string }>(EVENT.anchor, (e) =>
    handler(e.payload),
  );
}

export async function listenAnchorClear(handler: () => void): Promise<UnlistenFn> {
  return listen<null>(EVENT.clearAnchor, () => handler());
}
