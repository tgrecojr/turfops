import { useCallback, useEffect, useState } from 'react';
import { getDashboard } from '../api/client';
import AlertCard from '../components/AlertCard';
import Gauge from '../components/Gauge';
import type { DashboardResponse } from '../types';
import { APPLICATION_TYPE_COLORS, APPLICATION_TYPE_LABELS } from '../types';

const POLL_INTERVAL = 30_000; // 30 seconds

export default function Dashboard() {
  const [data, setData] = useState<DashboardResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchData = useCallback(async () => {
    try {
      const d = await getDashboard();
      setData(d);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load dashboard');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
    const id = setInterval(fetchData, POLL_INTERVAL);
    return () => clearInterval(id);
  }, [fetchData]);

  if (loading) return <div role="status" style={styles.loading}>Loading dashboard...</div>;
  if (error && !data) return <div role="alert" style={styles.error}>Error: {error}</div>;
  if (!data) return null;

  const { profile, environmental, recommendations, recent_applications, connections } = data;
  const current = environmental.current;

  return (
    <div>
      {error && (
        <div role="alert" style={styles.errorBanner}>
          Data may be stale: {error}
        </div>
      )}
      <div style={styles.headerRow}>
        <h1 style={styles.title}>{profile.name}</h1>
        <div style={styles.meta}>
          {profile.grass_type} &middot; Zone {profile.usda_zone}
          {environmental.last_updated && (
            <span style={styles.updated}>
              {' '}
              &middot; Updated{' '}
              {new Date(environmental.last_updated).toLocaleTimeString()}
            </span>
          )}
        </div>
      </div>

      {/* Connection indicators */}
      <div style={styles.connections}>
        <ConnectionDot label="SoilData" ok={connections.soildata} />
        <ConnectionDot label="Home Assistant" ok={connections.homeassistant} />
        <ConnectionDot label="OpenWeatherMap" ok={connections.openweathermap} />
      </div>

      {/* Gauges */}
      <div style={styles.gaugeGrid}>
        <div style={styles.card}>
          <Gauge
            label="Soil Temp (10cm)"
            value={current?.soil_temp_10_f ?? null}
            unit="°F"
            min={30}
            max={100}
            thresholds={{ warn: 75, critical: 85 }}
          />
          {environmental.soil_temp_7day_avg_f !== null && (
            <div style={styles.subtext}>
              7-day avg: {environmental.soil_temp_7day_avg_f.toFixed(1)}°F
              {' '}{trendArrow(environmental.soil_temp_trend)}
            </div>
          )}
        </div>
        <div style={styles.card}>
          <Gauge
            label="Ambient Temp"
            value={current?.ambient_temp_f ?? null}
            unit="°F"
            min={0}
            max={110}
            thresholds={{ warn: 85, critical: 95 }}
          />
          {environmental.ambient_temp_7day_avg_f !== null && (
            <div style={styles.subtext}>
              7-day avg: {environmental.ambient_temp_7day_avg_f.toFixed(1)}°F
            </div>
          )}
        </div>
        <div style={styles.card}>
          <Gauge
            label="Humidity"
            value={current?.humidity_percent ?? null}
            unit="%"
            min={0}
            max={100}
            thresholds={{ warn: 80, critical: 90 }}
          />
        </div>
        <div style={styles.card}>
          <Gauge
            label="Soil Moisture (10cm)"
            value={
              current?.soil_moisture_10 !== null && current?.soil_moisture_10 !== undefined
                ? current.soil_moisture_10 * 100
                : null
            }
            unit="%"
            min={0}
            max={50}
            thresholds={{ warn: 40, critical: 45 }}
          />
          {environmental.precipitation_7day_total_mm !== null && (
            <div style={styles.subtext}>
              7-day precip: {environmental.precipitation_7day_total_mm.toFixed(1)} mm
            </div>
          )}
        </div>
      </div>

      {/* Alerts & Recent Apps side by side */}
      <div style={styles.twoCol}>
        <div style={{ flex: 1 }}>
          <h2 style={styles.sectionTitle}>Active Alerts</h2>
          {recommendations.length === 0 ? (
            <div style={styles.empty}>No active recommendations</div>
          ) : (
            recommendations.map((r) => <AlertCard key={r.id} rec={r} />)
          )}
        </div>
        <div style={{ flex: 1 }}>
          <h2 style={styles.sectionTitle}>Recent Applications</h2>
          {recent_applications.length === 0 ? (
            <div style={styles.empty}>No applications recorded</div>
          ) : (
            <table style={styles.table}>
              <thead>
                <tr>
                  <th style={styles.th}>Date</th>
                  <th style={styles.th}>Type</th>
                  <th style={styles.th}>Product</th>
                </tr>
              </thead>
              <tbody>
                {recent_applications.map((app) => (
                  <tr key={app.id}>
                    <td style={styles.td}>{app.application_date}</td>
                    <td style={styles.td}>
                      <span
                        style={{
                          ...styles.typeBadge,
                          backgroundColor:
                            APPLICATION_TYPE_COLORS[app.application_type] + '22',
                          color: APPLICATION_TYPE_COLORS[app.application_type],
                          borderColor: APPLICATION_TYPE_COLORS[app.application_type],
                        }}
                      >
                        {APPLICATION_TYPE_LABELS[app.application_type]}
                      </span>
                    </td>
                    <td style={styles.td}>{app.product_name || '-'}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}

function ConnectionDot({ label, ok }: { label: string; ok: boolean }) {
  return (
    <span style={styles.connItem}>
      <span
        style={{
          ...styles.dot,
          backgroundColor: ok ? '#48bb78' : '#a0aec0',
        }}
      />
      {label}
    </span>
  );
}

function trendArrow(trend: string): string {
  switch (trend) {
    case 'Rising':
      return '↑';
    case 'Falling':
      return '↓';
    case 'Stable':
      return '→';
    default:
      return '';
  }
}

const styles: Record<string, React.CSSProperties> = {
  loading: { padding: '2rem', color: '#718096' },
  error: { padding: '2rem', color: '#e53e3e' },
  errorBanner: {
    padding: '0.5rem 1rem',
    backgroundColor: '#fed7d7',
    color: '#c53030',
    borderRadius: 6,
    marginBottom: '1rem',
    fontSize: '0.85rem',
  },
  headerRow: { marginBottom: '1rem' },
  title: { margin: 0, fontSize: '1.5rem', color: '#1a202c' },
  meta: { color: '#718096', fontSize: '0.85rem', marginTop: 4 },
  updated: { color: '#a0aec0' },
  connections: {
    display: 'flex',
    gap: 16,
    marginBottom: '1.2rem',
    fontSize: '0.8rem',
    color: '#4a5568',
  },
  connItem: { display: 'flex', alignItems: 'center', gap: 6 },
  dot: {
    width: 8,
    height: 8,
    borderRadius: '50%',
    display: 'inline-block',
  },
  gaugeGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
    gap: '1rem',
    marginBottom: '1.5rem',
  },
  card: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '1rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  subtext: { fontSize: '0.75rem', color: '#718096', marginTop: 4 },
  twoCol: { display: 'flex', gap: '1.5rem', flexWrap: 'wrap' as const },
  sectionTitle: {
    fontSize: '1rem',
    fontWeight: 600,
    color: '#2d3748',
    marginBottom: '0.75rem',
  },
  empty: {
    color: '#a0aec0',
    fontSize: '0.85rem',
    padding: '1rem',
    backgroundColor: '#fff',
    borderRadius: 8,
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse' as const,
    backgroundColor: '#fff',
    borderRadius: 8,
    overflow: 'hidden',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  th: {
    textAlign: 'left' as const,
    padding: '0.5rem 0.75rem',
    fontSize: '0.75rem',
    color: '#718096',
    fontWeight: 600,
    borderBottom: '1px solid #e2e8f0',
    textTransform: 'uppercase' as const,
  },
  td: {
    padding: '0.5rem 0.75rem',
    fontSize: '0.85rem',
    borderBottom: '1px solid #edf2f7',
    color: '#2d3748',
  },
  typeBadge: {
    display: 'inline-block',
    padding: '2px 8px',
    borderRadius: 10,
    fontSize: '0.75rem',
    fontWeight: 600,
    border: '1px solid',
  },
};
