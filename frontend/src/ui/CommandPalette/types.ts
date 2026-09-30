import type { ReactNode } from 'react';

export interface CommandItem {
  id: string;
  title: string;
  subtitle?: string;
  icon?: ReactNode;
  onSelect: () => void;
}

export interface CommandSource {
  id: string;
  /** Group heading shown above this source's results. */
  label: string;
  search: (query: string) => Promise<CommandItem[]>;
}
