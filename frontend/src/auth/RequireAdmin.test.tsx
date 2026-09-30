import { render, screen } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import { AuthContext } from '../contexts/AuthContext';
import { RequireAdmin } from './RequireAdmin';

function renderAs(role: string | null, fallback?: React.ReactNode) {
  const user = role ? ({ id: '1', username: 'ada', email: 'a@example.com', role, is_active: true } as never) : null;
  return render(
    <AuthContext.Provider value={{ user } as never}>
      <MemoryRouter initialEntries={['/admin']}>
        <Routes>
          <Route
            path="/admin"
            element={
              <RequireAdmin fallback={fallback}>
                <p>admin area</p>
              </RequireAdmin>
            }
          />
          <Route path="/board" element={<p>board</p>} />
        </Routes>
      </MemoryRouter>
    </AuthContext.Provider>,
  );
}

describe('RequireAdmin', () => {
  it('renders its children for an admin', () => {
    renderAs('admin');
    expect(screen.getByText('admin area')).toBeInTheDocument();
  });

  it('sends other users home by default', () => {
    renderAs('user');
    expect(screen.getByText('board')).toBeInTheDocument();
    expect(screen.queryByText('admin area')).not.toBeInTheDocument();
  });

  it('renders the fallback instead when one is given', () => {
    renderAs(null, <p>admins only</p>);
    expect(screen.getByText('admins only')).toBeInTheDocument();
    expect(screen.queryByText('admin area')).not.toBeInTheDocument();
  });

  it('renders nothing for a null fallback', () => {
    const { container } = renderAs('user', null);
    expect(container).toBeEmptyDOMElement();
  });
});
