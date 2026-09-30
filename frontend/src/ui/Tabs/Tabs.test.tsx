import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { Tab, TabList, TabPanel, Tabs } from './Tabs';

const setup = () =>
  render(
    <Tabs>
      <TabList aria-label="Sections">
        <Tab id="one">One</Tab>
        <Tab id="two">Two</Tab>
        <Tab id="three">Three</Tab>
      </TabList>
      <TabPanel id="one">Panel one</TabPanel>
      <TabPanel id="two">Panel two</TabPanel>
      <TabPanel id="three">Panel three</TabPanel>
    </Tabs>,
  );

describe('Tabs', () => {
  it('renders a named tablist with the first tab selected', () => {
    setup();
    expect(screen.getByRole('tablist', { name: 'Sections' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'One' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tabpanel')).toHaveTextContent('Panel one');
  });

  it('moves selection with arrow keys', async () => {
    const user = userEvent.setup();
    setup();
    await user.tab();
    await user.keyboard('{ArrowRight}');
    expect(screen.getByRole('tab', { name: 'Two' })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tabpanel')).toHaveTextContent('Panel two');
    await user.keyboard('{ArrowLeft}');
    expect(screen.getByRole('tab', { name: 'One' })).toHaveAttribute('aria-selected', 'true');
  });

  it('selects on click', async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole('tab', { name: 'Three' }));
    expect(screen.getByRole('tabpanel')).toHaveTextContent('Panel three');
  });
});
