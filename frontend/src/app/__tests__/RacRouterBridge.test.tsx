import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { expect, it } from 'vitest';
import { ButtonLink } from '../../ui';
import { RacRouterBridge } from '../RacRouterBridge';

function Where() {
  const { pathname, search } = useLocation();
  return <output aria-label="location">{pathname + search}</output>;
}

it('lets React Aria links (ButtonLink, DocumentCard) navigate inside the app', async () => {
  render(
    <MemoryRouter initialEntries={['/home']}>
      <RacRouterBridge>
        <ButtonLink href="/sources?section=connections&new=1" variant="secondary">
          Connect source
        </ButtonLink>
        <Where />
      </RacRouterBridge>
    </MemoryRouter>,
  );
  const link = screen.getByRole('link', { name: 'Connect source' });
  expect(link).toHaveAttribute('href', '/sources?section=connections&new=1');
  await userEvent.setup().click(link);
  expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent('/sources?section=connections&new=1');
});
