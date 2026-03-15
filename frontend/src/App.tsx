import { Component, lazy, Suspense, type ReactNode } from 'react';
import { BrowserRouter, Link, Route, Routes } from 'react-router-dom';
import Layout from './components/Layout';

const Dashboard = lazy(() => import('./pages/Dashboard'));
const Applications = lazy(() => import('./pages/Applications'));
const Calendar = lazy(() => import('./pages/Calendar'));
const Environmental = lazy(() => import('./pages/Environmental'));
const Recommendations = lazy(() => import('./pages/Recommendations'));
const Settings = lazy(() => import('./pages/Settings'));

class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: '2rem', fontFamily: 'system-ui' }}>
          <h1 style={{ color: '#e53e3e', fontSize: '1.5rem' }}>
            Something went wrong
          </h1>
          <p style={{ color: '#4a5568', margin: '1rem 0' }}>
            {this.state.error.message}
          </p>
          <button
            onClick={() => {
              this.setState({ error: null });
              window.location.href = '/';
            }}
            style={{
              padding: '0.5rem 1rem',
              backgroundColor: '#3182ce',
              color: '#fff',
              border: 'none',
              borderRadius: 6,
              cursor: 'pointer',
            }}
          >
            Return to Dashboard
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

function NotFound() {
  return (
    <div style={{ padding: '2rem', textAlign: 'center' }}>
      <h1 style={{ fontSize: '1.5rem', color: '#2d3748' }}>Page Not Found</h1>
      <p style={{ color: '#718096', margin: '1rem 0' }}>
        The page you're looking for doesn't exist.
      </p>
      <Link
        to="/"
        style={{ color: '#3182ce', textDecoration: 'underline' }}
      >
        Go to Dashboard
      </Link>
    </div>
  );
}

const routeFallback = (
  <div style={{ padding: '2rem', color: '#718096' }}>Loading...</div>
);

export default function App() {
  return (
    <ErrorBoundary>
      <BrowserRouter>
        <Suspense fallback={routeFallback}>
          <Routes>
            <Route element={<Layout />}>
              <Route index element={<Dashboard />} />
              <Route path="applications" element={<Applications />} />
              <Route path="calendar" element={<Calendar />} />
              <Route path="environmental" element={<Environmental />} />
              <Route path="recommendations" element={<Recommendations />} />
              <Route path="settings" element={<Settings />} />
              <Route path="*" element={<NotFound />} />
            </Route>
          </Routes>
        </Suspense>
      </BrowserRouter>
    </ErrorBoundary>
  );
}
