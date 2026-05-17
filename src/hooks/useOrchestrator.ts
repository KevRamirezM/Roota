import { useCallback, useReducer, useRef } from "react";

import { celebrateTaskComplete } from "../lib/celebrate";
import {
  cancelSession,
  confirmResponse,
  newOrchestratorChannel,
  requestStuckHelp as requestStuckHelpInvoke,
  startSession,
} from "../tauri-api";
import type { AppPhase, GuideStep, OrchestratorEvent } from "../types";

type Action =
  | { type: "submit" }
  | { type: "event"; event: OrchestratorEvent }
  | { type: "reject_confirmation" }
  | { type: "reset" };

function reducer(state: AppPhase, action: Action): AppPhase {
  switch (action.type) {
    case "submit":
      return { kind: "classifying" };
    case "reset":
      return { kind: "idle" };
    case "reject_confirmation":
      return { kind: "cancelled" };
    case "event": {
      const e = action.event;
      switch (e.kind) {
        case "ConfirmationRequested":
          return { kind: "awaiting_confirmation", message: e.data.message };
        case "Observing":
          return { kind: "observing", pass: e.data.pass };
        case "PlanPreview":
          return {
            kind: "plan_preview",
            summary: e.data.summary,
            steps: e.data.steps,
          };
        case "Replanning":
          return { kind: "replanning", reason: e.data.reason };
        case "StepReady": {
          const step = e.data.step as GuideStep;
          const anchorFromStep =
            step.anchorXy != null
              ? { x: step.anchorXy[0], y: step.anchorXy[1], label: step.targetText }
              : null;
          return {
            kind: "running",
            step,
            anchor: anchorFromStep,
            stepSuccess: false,
          };
        }
        case "StepCompleted":
          if (state.kind === "running") {
            return { ...state, stepSuccess: true };
          }
          return state;
        case "AnchorChanged":
          if (state.kind === "running") {
            return {
              ...state,
              anchor: { x: e.data.x, y: e.data.y, label: e.data.label },
              stepSuccess: false,
            };
          }
          return state;
        case "GoalCompleted":
          return { kind: "completed" };
        case "Error":
          return { kind: "error", message: e.data.message };
        case "Finished":
          if (
            state.kind === "completed" ||
            state.kind === "error" ||
            state.kind === "cancelled" ||
            state.kind === "running"
          ) {
            return state;
          }
          if (state.kind === "awaiting_confirmation") {
            return { kind: "cancelled" };
          }
          return { kind: "idle" };
      }
    }
  }
}

export function useOrchestrator() {
  const [phase, dispatch] = useReducer(reducer, { kind: "idle" } as AppPhase);
  const channelRef = useRef<ReturnType<typeof newOrchestratorChannel> | null>(null);

  const submit = useCallback(async (utterance: string) => {
    dispatch({ type: "submit" });
    const channel = newOrchestratorChannel();
    channel.onmessage = (msg) => {
      if (msg.kind === "GoalCompleted") {
        celebrateTaskComplete();
      }
      dispatch({ type: "event", event: msg });
    };
    channelRef.current = channel;
    try {
      await startSession(utterance, channel);
    } catch (err) {
      dispatch({
        type: "event",
        event: { kind: "Error", data: { message: String(err) } },
      });
    }
  }, []);

  const respondToConfirmation = useCallback(async (accepted: boolean) => {
    if (!accepted) {
      dispatch({ type: "reject_confirmation" });
    }
    await confirmResponse(accepted);
  }, []);

  const cancel = useCallback(async () => {
    try {
      await cancelSession();
    } finally {
      dispatch({ type: "reset" });
    }
  }, []);

  const requestStuckHelp = useCallback(async () => {
    try {
      await requestStuckHelpInvoke();
    } catch (err) {
      dispatch({
        type: "event",
        event: { kind: "Error", data: { message: String(err) } },
      });
    }
  }, []);

  const busy =
    phase.kind === "classifying" ||
    phase.kind === "observing" ||
    phase.kind === "plan_preview" ||
    phase.kind === "replanning" ||
    phase.kind === "running" ||
    phase.kind === "awaiting_confirmation";

  return { phase, submit, respondToConfirmation, cancel, requestStuckHelp, busy };
}
