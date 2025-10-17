import { NavLink, Outlet } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';
import LoginPrompt from './LoginPrompt';
import './AppShell.css';
import StatusPill from './StatusPill';

const navItems = [
  { path: '/', label: 'Employee Portal' },
  { path: '/manager', label: 'Manager Console' },
  { path: '/finance', label: 'Finance Console' }
];

const AppShell = () => {
  const { isAuthenticated, isReady } = useAuth();

  if (!isReady) {
    return (
      <div className="app-shell__loading" role="status">
        Checking session…
      </div>
    );
  }

  if (!isAuthenticated) {
    return (
      <div className="app-shell__login">
        <div className="app-shell__login-card">
          <header className="app-shell__brand">
            <img src="/fsi-logo (1).png" alt="Freight Services logo" />
            <h1>FSI Expenses</h1>
          </header>
          <LoginPrompt />
        </div>
      </div>
    );
  }

  return (
    <div className="app-shell">
      <aside className="app-shell__sidebar">
        <header className="app-shell__brand">
          <img src="/fsi-logo (1).png" alt="Freight Services logo" />
          <h1>FSI Expenses</h1>
        </header>
        <nav>
          {navItems.map((item) => (
            <NavLink key={item.path} to={item.path} end className={({ isActive }) => (isActive ? 'active' : '')}>
              {item.label}
            </NavLink>
          ))}
        </nav>
        <footer>
          <StatusPill status="online" />
          <small>Version: {__BUILD_VERSION__}</small>
        </footer>
      </aside>
      <main className="app-shell__content">
        <Outlet />
      </main>
    </div>
  );
};

export default AppShell;
