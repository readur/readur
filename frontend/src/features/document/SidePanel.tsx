import type { Key } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { SlideOver, Tab, TabList, TabPanel, Tabs } from '../../ui';
import { CommentsPanel } from './comments/CommentsPanel';
import { SharedLinksManager } from './sharing/SharedLinksManager';

export type SidePanelTab = 'comments' | 'links';

export interface SidePanelProps {
  documentId: string;
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  tab: SidePanelTab;
  onTabChange: (tab: SidePanelTab) => void;
}

/** Right-hand panel with the document's comments and its share links. */
export function SidePanel({ documentId, isOpen, onOpenChange, tab, onTabChange }: SidePanelProps) {
  const { t } = useTranslation();
  return (
    <SlideOver
      title={tab === 'comments' ? t('document.panel.comments', 'Comments') : t('document.panel.links', 'Share links')}
      isOpen={isOpen}
      onOpenChange={onOpenChange}
    >
      <Tabs selectedKey={tab} onSelectionChange={(key: Key) => onTabChange(key as SidePanelTab)}>
        <TabList aria-label={t('document.panel.label', 'Side panel')}>
          <Tab id="comments">{t('document.panel.comments', 'Comments')}</Tab>
          <Tab id="links">{t('document.panel.links', 'Share links')}</Tab>
        </TabList>
        <TabPanel id="comments">
          <CommentsPanel documentId={documentId} />
        </TabPanel>
        <TabPanel id="links">
          <SharedLinksManager documentId={documentId} />
        </TabPanel>
      </Tabs>
    </SlideOver>
  );
}
