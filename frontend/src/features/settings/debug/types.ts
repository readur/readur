/* The debug endpoint returns loosely-typed diagnostics; these mirror what the UI reads. */
/* eslint-disable @typescript-eslint/no-explicit-any */

export interface DebugStep {
  step: number;
  name: string;
  status: string;
  details: any;
  success: boolean;
  error?: string;
}

export interface DebugInfo {
  document_id: string;
  filename: string;
  overall_status: string;
  pipeline_steps: DebugStep[];
  failed_document_info?: any;
  user_settings: any;
  debug_timestamp: string;
  detailed_processing_logs?: any[];
  file_analysis?: any;
}

export type StepState = 'processing' | 'completed' | 'failed' | 'pending' | 'warning' | 'other';

/** Collapses the pipeline's many status words into the shapes the UI draws. */
export function stepState(status: string, success: boolean): StepState {
  if (status === 'processing') return 'processing';
  if (success || status === 'completed' || status === 'passed') return 'completed';
  if (status === 'failed' || status === 'error') return 'failed';
  if (status === 'pending' || status === 'not_reached') return 'pending';
  if (status === 'not_queued' || status === 'ocr_disabled') return 'warning';
  return 'other';
}

export const STATE_GLYPH: Record<StepState, string> = {
  processing: '◐',
  completed: '■',
  failed: '▲',
  pending: '○',
  warning: '◆',
  other: '▶',
};

export const mb = (bytes: number | undefined | null) => `${((bytes ?? 0) / 1024 / 1024).toFixed(2)} MB`;
export const when = (iso: string | null | undefined, fallback = '-') => (iso ? new Date(iso).toLocaleString() : fallback);
