export type ActionVerb = "click" | "double_click" | "right_click" | "type" | "locate";

export interface OverlayRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface GuideStep {
  index: number;
  total: number;
  action: ActionVerb;
  targetText: string;
  instruction: string;
  anchorXy: [number, number] | null;
  anchorBounds: [number, number, number, number] | null;
}

export interface GuidancePayload {
  active: boolean;
  instruction: string;
  action: ActionVerb;
  stepIndex: number;
  stepTotal: number;
  targetLabel: string;
  clickHint: string;
  hasTarget: boolean;
  rect: OverlayRect | null;
}

export interface AnchorState {
  x: number;
  y: number;
  label: string;
}

export type OrchestratorEvent =
  | { kind: "ConfirmationRequested"; data: { message: string } }
  | { kind: "StepReady"; data: { step: GuideStep } }
  | { kind: "StepCompleted"; data: { index: number } }
  | { kind: "AnchorChanged"; data: { x: number; y: number; label: string } }
  | { kind: "GoalCompleted"; data: { steps: number } }
  | { kind: "Error"; data: { message: string } }
  | { kind: "Finished"; data: null };

export type AppPhase =
  | { kind: "idle" }
  | { kind: "classifying" }
  | { kind: "awaiting_confirmation"; message: string }
  | {
      kind: "running";
      step: GuideStep;
      anchor: AnchorState | null;
      stepSuccess: boolean;
    }
  | { kind: "completed" }
  | { kind: "cancelled" }
  | { kind: "error"; message: string };
