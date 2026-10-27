import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ControlInteractionKind } from "../bindings/ControlInteractionKind";
import actions from "./control-actions.json";

type Activity = { action: string; kind: ControlInteractionKind };
const descriptions: Record<string, string> = actions;
const pending: Activity[] = [];
const maximumPending = 1024;
let draining = false;
let failed = false;
let cleanup: (() => void) | undefined;

function failure() {
  if (!failed) console.error("Could not record application interaction");
  failed = true;
}

async function drain() {
  if (draining) return;
  draining = true;
  while (pending.length) {
    const activity = pending.shift()!;
    try {
      await invoke("log_control_interaction", activity);
      failed = false;
    } catch {
      failure();
    }
  }
  draining = false;
}

export function recordControlAction(action: string, kind: ControlInteractionKind): void {
  if (!Object.hasOwn(descriptions, action)) return;
  if (!isTauri()) {
    console.info(`UI ${kind} ${action}: ${descriptions[action]}`);
    return;
  }
  if (pending.length + Number(draining) >= maximumPending) {
    failure();
    return;
  }
  pending.push({ action, kind });
  void drain();
}

/** Observe controls across React roots and portals without inspecting their values or labels. */
export function installControlActivity(): () => void {
  if (cleanup) return cleanup;
  const edited = new WeakSet<Element>();
  const selector = "button, a, summary, input, textarea, select, [role=button], [role=menuitem], [role=tab], [role=option], [role=switch], [role=checkbox], [role=radio], [contenteditable=true]";
  const element = (event: Event) => event.target instanceof Element ? event.target.closest<HTMLElement>(selector) : null;
  const action = (target: HTMLElement) => target.closest<HTMLElement>("[data-control-action]")?.dataset.controlAction;
  const excluded = (target: HTMLElement) => target.closest("[data-control-semantic]") !== null || target.matches(":disabled, [aria-disabled=true]");
  const editing = (target: HTMLElement) => target instanceof HTMLTextAreaElement || target.isContentEditable || (target instanceof HTMLInputElement && !["checkbox", "radio", "file", "button", "submit", "reset"].includes(target.type));
  const emit = (target: HTMLElement, kind: ControlInteractionKind) => {
    const id = action(target);
    if (id && !excluded(target)) recordControlAction(id, kind);
  };
  const click = (event: Event) => {
    const target = element(event);
    if (!target || target instanceof HTMLInputElement && (["checkbox", "radio", "file"].includes(target.type) || target.type === "submit" && !!target.form) || target instanceof HTMLButtonElement && target.type === "submit" && !!target.form || target instanceof HTMLSelectElement || editing(target)) return;
    emit(target, "activate");
  };
  const input = (event: Event) => {
    const target = element(event);
    if (target && editing(target) && !excluded(target)) edited.add(target);
  };
  const change = (event: Event) => {
    const target = element(event);
    if (!target || excluded(target)) return;
    if (editing(target)) {
      if (!edited.has(target)) return;
      edited.delete(target);
    }
    emit(target, "change");
  };
  const blur = (event: Event) => {
    const target = element(event);
    if (target && edited.has(target)) {
      edited.delete(target);
      emit(target, "change");
    }
  };
  const submit = (event: Event) => {
    if (!(event.target instanceof HTMLElement)) return;
    const submitter = event instanceof SubmitEvent && event.submitter instanceof HTMLElement ? event.submitter : event.target;
    emit(submitter, "submit");
  };
  document.addEventListener("click", click, true);
  document.addEventListener("input", input, true);
  document.addEventListener("change", change, true);
  document.addEventListener("focusout", blur, true);
  document.addEventListener("submit", submit, true);
  cleanup = () => {
    document.removeEventListener("click", click, true);
    document.removeEventListener("input", input, true);
    document.removeEventListener("change", change, true);
    document.removeEventListener("focusout", blur, true);
    document.removeEventListener("submit", submit, true);
    cleanup = undefined;
  };
  return cleanup;
}
