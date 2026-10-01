import { useState, type ReactNode } from 'react';
import { Switch } from '../../../ui';
import type { SettingKey, SettingsPatch } from './settingsModel';

export interface SettingSwitchProps {
  settingKey: SettingKey;
  label: string;
  description?: ReactNode;
  isSelected: boolean;
  onSave: (patch: SettingsPatch) => Promise<void>;
  isDisabled?: boolean;
}

/** A switch that takes effect immediately: it saves on change (the save toasts) and reverts on failure. */
export function SettingSwitch({ settingKey, label, description, isSelected, onSave, isDisabled }: SettingSwitchProps) {
  const [pending, setPending] = useState<boolean | null>(null);
  const shown = pending ?? isSelected;
  return (
    <Switch
      label={label}
      description={description}
      isSelected={shown}
      isDisabled={isDisabled || pending !== null}
      onChange={async (next) => {
        setPending(next);
        try {
          await onSave({ [settingKey]: next } as SettingsPatch);
        } catch {
          /* the save already toasted the error; fall back to the saved value */
        } finally {
          setPending(null);
        }
      }}
    />
  );
}
