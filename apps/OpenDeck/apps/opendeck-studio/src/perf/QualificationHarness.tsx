import { useEffect, useRef, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { bridge, type QualificationPhase } from '../bridge';
import type { Workspace } from '../model/workspace';
import { clearMetrics, metricSnapshot, recordLatency } from './metrics';

interface QualificationHarnessProps {
  enabled: boolean;
  phase: QualificationPhase;
  workspace: Workspace;
  startedAtMs?: number;
  tauriSetupMs?: number;
}

const WEBVIEW_JS_AT_MS = performance.timeOrigin + performance.now();

type Box = { x: number; y: number; width: number; height: number };

function boxFor(element: Element): Box {
  const rect = element.getBoundingClientRect();
  return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
}

function nextFrame(): Promise<number> {
  return new Promise((resolve) => requestAnimationFrame(resolve));
}

async function settleFrames(count = 2): Promise<void> {
  for (let i = 0; i < count; i += 1) await nextFrame();
}

function elementBySelector(selector: string): HTMLElement {
  const element = document.querySelector<HTMLElement>(selector);
  if (!element) throw new Error(`Qualification element missing: ${selector}`);
  return element;
}

async function measuredAction(name: string, action: () => void): Promise<void> {
  const started = performance.now();
  action();
  await settleFrames(1);
  recordLatency(name, performance.now() - started);
}

function changeSelect(select: HTMLSelectElement, value: string): void {
  const descriptor = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value');
  descriptor?.set?.call(select, value);
  select.dispatchEvent(new Event('change', { bubbles: true }));
}

function changeInput(input: HTMLInputElement, value: string): void {
  const descriptor = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value');
  descriptor?.set?.call(input, value);
  input.dispatchEvent(new Event('input', { bubbles: true }));
  input.dispatchEvent(new Event('change', { bubbles: true }));
}

function geometryPayload(): Record<string, unknown> {
  const regionNames = ['header', 'sidebar', 'main', 'device', 'configuration', 'actions'];
  const regions = Object.fromEntries(regionNames.map((name) => {
    const node = document.querySelector(`[data-layout-region="${name}"]`);
    if (!node) throw new Error(`Missing layout region: ${name}`);
    return [name, boxFor(node)];
  }));
  const elements = Object.fromEntries(['device', 'key', 'touch', 'dial'].map((name) => [name, [...document.querySelectorAll(`[data-qualify-element="${name}"]`)].map(boxFor)]));
  return {
    schemaVersion: 1,
    release: '2.0.48',
    viewport: { width: window.innerWidth, height: window.innerHeight, devicePixelRatio: window.devicePixelRatio },
    regions,
    elements,
  };
}

async function framePacingSample(): Promise<{ total: number; within16_7: number; within33_3: number; over50: number; maxMs: number }> {
  const deltas: number[] = [];
  const dials = [...document.querySelectorAll<HTMLElement>('[data-testid="dial"]')];
  let previous = await nextFrame();
  for (let i = 0; i < 600; i += 1) {
    if (dials.length >= 2 && i % 6 === 0) dials[(i / 6) % 2].click();
    const now = await nextFrame();
    deltas.push(now - previous);
    previous = now;
  }
  return {
    total: deltas.length,
    within16_7: deltas.filter((value) => value <= 16.7).length,
    within33_3: deltas.filter((value) => value <= 33.3).length,
    over50: deltas.filter((value) => value > 50).length,
    maxMs: Math.max(...deltas),
  };
}

async function benchmarkUi(workspace: Workspace): Promise<Record<string, unknown>> {
  clearMetrics();
  await settleFrames(3);
  const sidebarButtons = [elementBySelector('[aria-label="Buttons"]'), elementBySelector('[aria-label="Dials"]')];
  const dials = [...document.querySelectorAll<HTMLElement>('[data-testid="dial"]')];
  const tabs = [...document.querySelectorAll<HTMLElement>('[role="tab"]')].filter((node) => node.textContent === 'Keys' || node.textContent === 'Dials');
  if (dials.length < 2 || tabs.length < 2) throw new Error('Qualification controls are incomplete');

  for (let i = 0; i < 100; i += 1) await measuredAction('sidebarSelectionMs', () => sidebarButtons[i % 2].click());
  for (let i = 0; i < 100; i += 1) await measuredAction('controlToConfigurationMs', () => dials[i % 2].click());
  for (let i = 0; i < 100; i += 1) await measuredAction('keysDialsSwitchMs', () => tabs[i % 2].click());

  const search = elementBySelector('[aria-label="Search actions"]') as HTMLInputElement;
  for (let i = 0; i < 100; i += 1) {
    await measuredAction('search5000Ms', () => changeInput(search, `Synthetic ${i % 2 ? '19' : '27'}`));
  }
  changeInput(search, '');
  await settleFrames(2);

  const assignments = ['Scene', 'Toggle Stream'];
  for (let i = 0; i < 100; i += 1) {
    const button = document.querySelector<HTMLElement>(`button[title^="${assignments[i % 2]}"]`);
    if (!button) throw new Error(`Assignment action not visible: ${assignments[i % 2]}`);
    await measuredAction('assignmentMs', () => button.click());
  }

  const profile = elementBySelector('[aria-label="Profile"]') as HTMLSelectElement;
  const profileValues = [...profile.options].map((option) => option.value);
  if (profileValues.length >= 2) {
    for (let i = 0; i < 100; i += 1) await measuredAction('profileSwitchMs', () => changeSelect(profile, profileValues[i % 2]));
  }

  for (let i = 0; i < 100; i += 1) {
    const started = performance.now();
    const pending = bridge.editorSaveWorkspace(workspace);
    recordLatency('persistenceEnqueueMs', performance.now() - started);
    await pending;
  }

  return { schemaVersion: 1, release: '2.0.48', interaction: metricSnapshot(), framePacing: await framePacingSample() };
}

export function QualificationHarness({ enabled, phase, workspace, startedAtMs, tauriSetupMs }: QualificationHarnessProps) {
  const [state, setState] = useState<'idle'|'running'|'hardware'|'complete'|'error'>('idle');
  const [hardwarePulse, setHardwarePulse] = useState(0);
  const workspaceRef = useRef(workspace);
  useEffect(() => {
    if (!enabled) return;
    let alive = true;
    let unlisten: UnlistenFn | undefined;
    if (phase === 'performance') {
      void listen('opendeck://hardware-input', () => {
        if (!alive) return;
        const started = performance.now();
        setHardwarePulse((value) => value + 1);
        requestAnimationFrame(() => recordLatency('hidBridgeToVisibleMs', performance.now() - started));
      }).then((cleanup) => { if (alive) unlisten = cleanup; else cleanup(); }).catch(() => undefined);
    }
    void (async () => {
      try {
        setState('running');
        if (phase === 'startup') {
          const origin = startedAtMs ?? performance.timeOrigin;
          const elapsed = () => Math.max(0, performance.timeOrigin + performance.now() - origin);
          const reactReadyMs = elapsed();
          const visualReadyMs = elapsed();
          await bridge.qualificationRecordUiMetrics({
            kind: 'startup',
            payload: {
              release: '2.0.48',
              processSpawnMs: 0,
              tauriSetupMs: tauriSetupMs ?? null,
              webviewJsMs: Math.max(0, WEBVIEW_JS_AT_MS - origin),
              reactReadyMs,
              workspaceReadyMs: reactReadyMs,
              visualReadyMs,
            },
          });
          if (alive) setState('complete');
          return;
        }
        const focusAck = await bridge.qualificationFocusWindow();
        if (focusAck.release !== '2.0.48') throw new Error(`Qualification focus release mismatch: ${focusAck.release}`);
        if ('fonts' in document) await document.fonts.ready;
        await settleFrames(4);
        await settleFrames(2);
        await bridge.qualificationRecordUiMetrics({ kind: 'visual', payload: geometryPayload() });
        if (phase === 'performance') {
          const performancePayload = await benchmarkUi(workspaceRef.current);
          if (alive) setState('hardware');
          await new Promise<void>((resolve) => window.setTimeout(resolve, 10_000));
          performancePayload.interaction = metricSnapshot();
          await bridge.qualificationRecordUiMetrics({ kind: 'performance', payload: performancePayload });
        }
        if (alive) setState('complete');
      } catch (error) {
        if (alive) setState('error');
        void bridge.qualificationRecordUiMetrics({ kind: 'error', payload: { message: String(error) } }).catch(() => undefined);
      }
    })();
    return () => { alive = false; unlisten?.(); };
  }, [enabled, phase, startedAtMs, tauriSetupMs]);
  if (!enabled) return null;
  if (phase === 'performance') return <output className="qualification-performance-state" data-qualification-state={state} data-hardware-pulse={hardwarePulse}>{state === 'hardware' ? 'Hardware latency: rotate or press any Stream Deck+ dial now…' : state === 'running' ? 'Running OpenDeck+ responsiveness qualification…' : state}</output>;
  return <output className="qualification-state sr-only" data-qualification-state={state}>{state}</output>;
}
