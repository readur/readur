import { useTranslation } from 'react-i18next';
import { Card, Tab, TabList, TabPanel, Tabs } from '../../../ui';
import { CopyBlock } from './CopyBlock';
import shared from '../shared/shared.module.css';
import styles from './ApiKeys.module.css';

/** Shown in the snippets until the user has a real key to paste in. */
export const API_KEY_PLACEHOLDER = '<your-api-key>';

export function mcpEndpoint(): string {
  return `${window.location.origin}/api/mcp`;
}

export function claudeCodeCommand(endpoint: string, apiKey: string): string {
  return `claude mcp add --transport http readur ${endpoint} --header "Authorization: Bearer ${apiKey}"`;
}

/**
 * Claude Desktop's config file only launches local (stdio) servers, so it reaches a remote server
 * through the `mcp-remote` bridge. The header value goes in `env` because a space inside an `args`
 * entry is split apart on some platforms.
 */
export function claudeDesktopJson(endpoint: string, apiKey: string): string {
  return JSON.stringify(
    {
      mcpServers: {
        readur: {
          command: 'npx',
          args: ['-y', 'mcp-remote', endpoint, '--header', 'Authorization:${READUR_AUTH}'],
          env: { READUR_AUTH: `Bearer ${apiKey}` },
        },
      },
    },
    null,
    2,
  );
}

/** For clients that speak Streamable HTTP natively and read an `mcpServers` file. */
export function mcpServersJson(endpoint: string, apiKey: string): string {
  return JSON.stringify(
    {
      mcpServers: {
        readur: { type: 'http', url: endpoint, headers: { Authorization: `Bearer ${apiKey}` } },
      },
    },
    null,
    2,
  );
}

/** Ready-to-paste client configuration for the MCP endpoint, for one API key (or a placeholder). */
export function McpSnippets({ apiKey = API_KEY_PLACEHOLDER }: { apiKey?: string }) {
  const { t } = useTranslation();
  const endpoint = mcpEndpoint();
  return (
    <div className={styles.snippets}>
      <CopyBlock label={t('settings.apiKeys.mcp.endpoint', 'MCP endpoint')} value={endpoint} />
      <Tabs defaultSelectedKey="claude-code">
        <TabList aria-label={t('settings.apiKeys.mcp.clients', 'Client configuration')}>
          <Tab id="claude-code">{t('settings.apiKeys.mcp.claudeCode', 'Claude Code')}</Tab>
          <Tab id="claude-desktop">{t('settings.apiKeys.mcp.claudeDesktop', 'Claude Desktop')}</Tab>
          <Tab id="json">{t('settings.apiKeys.mcp.json', 'Other clients')}</Tab>
        </TabList>
        <TabPanel id="claude-code" className={styles.snippetPanel}>
          <CopyBlock
            label={t('settings.apiKeys.mcp.command', 'Claude Code command')}
            value={claudeCodeCommand(endpoint, apiKey)}
          />
        </TabPanel>
        <TabPanel id="claude-desktop" className={styles.snippetPanel}>
          <p className={styles.snippetHelp}>
            {t(
              'settings.apiKeys.mcp.desktopHelp',
              'Add to claude_desktop_config.json, then restart Claude Desktop. It connects through the mcp-remote bridge, which needs Node.js.',
            )}
          </p>
          <CopyBlock
            label={t('settings.apiKeys.mcp.desktopConfig', 'Claude Desktop configuration')}
            value={claudeDesktopJson(endpoint, apiKey)}
            multiline
          />
        </TabPanel>
        <TabPanel id="json" className={styles.snippetPanel}>
          <p className={styles.snippetHelp}>
            {t(
              'settings.apiKeys.mcp.jsonHelp',
              'For clients that connect to remote servers over HTTP and read an mcpServers configuration file.',
            )}
          </p>
          <CopyBlock
            label={t('settings.apiKeys.mcp.config', 'MCP configuration')}
            value={mcpServersJson(endpoint, apiKey)}
            multiline
          />
        </TabPanel>
      </Tabs>
    </div>
  );
}

/** "Connect an AI assistant": how to point an MCP client at this server with an API key. */
export function McpConnectPanel() {
  const { t } = useTranslation();
  return (
    <Card as="section" aria-labelledby="mcp-connect-title">
      <h3 id="mcp-connect-title" className={styles.panelTitle}>
        {t('settings.apiKeys.mcp.title', 'Connect an AI assistant')}
      </h3>
      <p className={shared.sectionIntro}>
        {t(
          'settings.apiKeys.mcp.intro',
          'Readur speaks the Model Context Protocol, so assistants such as Claude can search, read, upload and label your documents. Create an API key above, then add this server to your client. The assistant can do anything the key can.',
        )}
      </p>
      <McpSnippets />
    </Card>
  );
}
