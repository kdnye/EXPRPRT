import { NavLink, Outlet } from 'react-router-dom';
import { useAuth } from '../hooks/useAuth';
import LoginPrompt from './LoginPrompt';
import './AppShell.css';
import StatusPill from './StatusPill';

const navSections: {
  label: string;
  description: string;
  items: { path: string; label: string; badge?: string }[];
}[] = [
  {
    label: 'Employee workspace',
    description: 'Capture receipts and submit new expenses.',
    items: [{ path: '/', label: 'Expense entry' }]
  },
  {
    label: 'Manager workspace',
    description: 'Review and approve employee submissions.',
    items: [{ path: '/manager', label: 'Expense approvals' }]
  },
  {
    label: 'Finance workspace',
    description: 'Finalize reimbursements and export ledgers.',
    items: [{ path: '/finance', label: 'Exports & tools', badge: 'Finance only' }]
  }
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
          {navSections.map((section) => (
            <section key={section.label} className="app-shell__nav-section">
              <p className="app-shell__nav-heading">{section.label}</p>
              <p className="app-shell__nav-description">{section.description}</p>
              {section.items.map((item) => (
                <NavLink key={item.path} to={item.path} end className={({ isActive }) => (isActive ? 'active' : '')}>
                  <span>{item.label}</span>
                  {item.badge ? <span className="app-shell__nav-badge">{item.badge}</span> : null}
                </NavLink>
              ))}
            </section>
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
