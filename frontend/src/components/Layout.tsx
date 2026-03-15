import { NavLink, Outlet } from 'react-router-dom';

const NAV_ITEMS = [
  { to: '/', label: 'Dashboard' },
  { to: '/applications', label: 'Applications' },
  { to: '/calendar', label: 'Calendar' },
  { to: '/environmental', label: 'Environmental' },
  { to: '/recommendations', label: 'Recommendations' },
  { to: '/seasonal-plan', label: 'Seasonal Plan' },
  { to: '/settings', label: 'Settings' },
];

export default function Layout() {
  return (
    <div style={{ display: 'flex', minHeight: '100vh' }}>
      <nav style={styles.sidebar} aria-label="Main navigation">
        <div style={styles.logo}>
          <span style={{ fontSize: '1.4rem' }}>TurfOps</span>
        </div>
        <ul style={styles.navList} role="list">
          {NAV_ITEMS.map((item) => (
            <li key={item.to}>
              <NavLink
                to={item.to}
                end={item.to === '/'}
                aria-current={undefined}
                style={({ isActive }) => ({
                  ...styles.navLink,
                  backgroundColor: isActive ? '#2d3748' : 'transparent',
                  color: isActive ? '#68d391' : '#cbd5e0',
                  fontWeight: isActive ? 700 : 400,
                })}
              >
                {item.label}
              </NavLink>
            </li>
          ))}
        </ul>
      </nav>
      <main style={styles.content} role="main">
        <Outlet />
      </main>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  sidebar: {
    width: 220,
    backgroundColor: '#1a202c',
    color: '#fff',
    display: 'flex',
    flexDirection: 'column',
    flexShrink: 0,
  },
  logo: {
    padding: '1.2rem 1rem',
    borderBottom: '1px solid #2d3748',
    fontWeight: 700,
  },
  navList: {
    listStyle: 'none',
    margin: 0,
    padding: '0.5rem 0',
  },
  navLink: {
    display: 'block',
    padding: '0.6rem 1rem',
    textDecoration: 'none',
    fontSize: '0.9rem',
    borderRadius: 4,
    margin: '2px 6px',
    transition: 'background-color 0.15s',
  },
  content: {
    flex: 1,
    padding: '1.5rem 2rem',
    backgroundColor: '#f7fafc',
    overflowY: 'auto' as const,
  },
};
