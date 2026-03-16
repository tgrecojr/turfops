import { useCallback, useEffect, useRef, useState } from 'react';
import { getDashboard, getGdd, getNitrogenBudget, getSoilTempForecast } from '../api/client';
import AlertCard from '../components/AlertCard';
import GddWidget from '../components/GddWidget';
import Gauge from '../components/Gauge';
import NitrogenBudgetWidget from '../components/NitrogenBudgetWidget';
import SoilTempForecastWidget from '../components/SoilTempForecastWidget';
import {
  SOIL_TEMP_GAUGE,
  AMBIENT_TEMP_GAUGE,
  HUMIDITY_GAUGE,
  SOIL_MOISTURE_GAUGE,
} from '../components/gaugeConfigs';
import { appTypeBadgeStyle, sharedStyles } from '../styles/shared';
import type { DashboardResponse, GddSummary, NitrogenBudget, SoilTempForecast } from '../types';
import { APPLICATION_TYPE_LABELS } from '../types';

const POLL_INTERVAL = 30_000; // 30 seconds

export default function Dashboard() {
  const [data, setData] = useState<DashboardResponse | null>(null);
  const [gddData, setGddData] = useState<GddSummary | null>(null);
  const [nBudget, setNBudget] = useState<NitrogenBudget | null>(null);
  const [soilForecast, setSoilForecast] = useState<SoilTempForecast | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const abortRef = useRef<AbortController | null>(null);

  const fetchData = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      const [d, gdd, nb, sf] = await Promise.all([
        getDashboard(),
        getGdd().catch(() => null),
        getNitrogenBudget().catch(() => null),
        getSoilTempForecast().catch(() => null),
      ]);
      if (!controller.signal.aborted) {
        setData(d);
        setGddData(gdd);
        setNBudget(nb);
        setSoilForecast(sf);
        setError(null);
      }
    } catch (e) {
      if (!controller.signal.aborted) {
        setError(e instanceof Error ? e.message : 'Failed to load dashboard');
      }
    } finally {
      if (!controller.signal.aborted) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    fetchData();

    const handleVisibility = () => {
      if (!document.hidden) fetchData();
    };

    let intervalId: ReturnType<typeof setInterval> | null = null;

    const startPolling = () => {
      stopPolling();
      if (!document.hidden) {
        intervalId = setInterval(fetchData, POLL_INTERVAL);
      }
    };

    const stopPolling = () => {
      if (intervalId !== null) {
        clearInterval(intervalId);
        intervalId = null;
      }
    };

    document.addEventListener('visibilitychange', handleVisibility);
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) stopPolling();
      else startPolling();
    });
    startPolling();

    return () => {
      stopPolling();
      abortRef.current?.abort();
      document.removeEventListener('visibilitychange', handleVisibility);
    };
  }, [fetchData]);

  if (loading) return <div role="status" style={sharedStyles.loading}>Loading dashboard...</div>;
  if (error && !data) return <div role="alert" style={sharedStyles.error}>Error: {error}</div>;
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
        <h1 style={sharedStyles.pageTitle}>{profile.name}</h1>
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
      <div style={sharedStyles.gaugeGrid}>
        <div style={sharedStyles.card}>
          <Gauge
            {...SOIL_TEMP_GAUGE}
            value={current?.soil_temp_10_f ?? null}
          />
          {environmental.soil_temp_7day_avg_f !== null && (
            <div style={styles.subtext}>
              7-day avg: {environmental.soil_temp_7day_avg_f.toFixed(1)}°F
              {' '}{trendArrow(environmental.soil_temp_trend)}
            </div>
          )}
        </div>
        <div style={sharedStyles.card}>
          <Gauge
            {...AMBIENT_TEMP_GAUGE}
            value={current?.ambient_temp_f ?? null}
          />
          {environmental.ambient_temp_7day_avg_f !== null && (
            <div style={styles.subtext}>
              7-day avg: {environmental.ambient_temp_7day_avg_f.toFixed(1)}°F
            </div>
          )}
        </div>
        <div style={sharedStyles.card}>
          <Gauge
            {...HUMIDITY_GAUGE}
            value={current?.humidity_percent ?? null}
          />
        </div>
        <div style={sharedStyles.card}>
          <Gauge
            {...SOIL_MOISTURE_GAUGE}
            value={
              current?.soil_moisture_10 !== null && current?.soil_moisture_10 !== undefined
                ? current.soil_moisture_10 * 100
                : null
            }
          />
          {environmental.precipitation_7day_total_mm !== null && (
            <div style={styles.subtext}>
              7-day precip: {environmental.precipitation_7day_total_mm.toFixed(1)} mm
            </div>
          )}
        </div>
      </div>

      {/* GDD, Nitrogen Budget & Soil Temp Forecast widgets */}
      {(gddData || nBudget || soilForecast) && (
        <div style={styles.widgetGrid}>
          {gddData && <GddWidget data={gddData} />}
          {nBudget && <NitrogenBudgetWidget data={nBudget} />}
          {soilForecast && (
            <SoilTempForecastWidget
              crossings={soilForecast.threshold_crossings}
              currentSoilTemp={current?.soil_temp_10_f ?? null}
            />
          )}
        </div>
      )}

      {/* Alerts & Recent Apps side by side */}
      <div style={styles.twoCol}>
        <div style={{ flex: 1 }}>
          <h2 style={sharedStyles.sectionTitle}>Active Alerts</h2>
          {recommendations.length === 0 ? (
            <div style={sharedStyles.empty}>No active recommendations</div>
          ) : (
            recommendations.map((r) => <AlertCard key={r.id} rec={r} />)
          )}
        </div>
        <div style={{ flex: 1 }}>
          <h2 style={sharedStyles.sectionTitle}>Recent Applications</h2>
          {recent_applications.length === 0 ? (
            <div style={sharedStyles.empty}>No applications recorded</div>
          ) : (
            <table style={sharedStyles.table}>
              <thead>
                <tr>
                  <th style={sharedStyles.th}>Date</th>
                  <th style={sharedStyles.th}>Type</th>
                  <th style={sharedStyles.th}>Product</th>
                </tr>
              </thead>
              <tbody>
                {recent_applications.map((app, index) => (
                  <tr key={app.id ?? `app-${index}`}>
                    <td style={sharedStyles.td}>{app.application_date}</td>
                    <td style={sharedStyles.td}>
                      <span
                        style={appTypeBadgeStyle(sharedStyles.badge, app.application_type)}
                      >
                        {APPLICATION_TYPE_LABELS[app.application_type]}
                      </span>
                    </td>
                    <td style={sharedStyles.td}>{app.product_name || '-'}</td>
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
      return '\u2191';
    case 'Falling':
      return '\u2193';
    case 'Stable':
      return '\u2192';
    default:
      return '';
  }
}

const styles: Record<string, React.CSSProperties> = {
  errorBanner: {
    padding: '0.5rem 1rem',
    backgroundColor: '#fed7d7',
    color: '#c53030',
    borderRadius: 6,
    marginBottom: '1rem',
    fontSize: '0.85rem',
  },
  headerRow: { marginBottom: '1rem' },
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
  subtext: { fontSize: '0.75rem', color: '#718096', marginTop: 4 },
  widgetGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
    gap: '1rem',
    marginBottom: '1.5rem',
  },
  twoCol: { display: 'flex', gap: '1.5rem', flexWrap: 'wrap' as const },
};
