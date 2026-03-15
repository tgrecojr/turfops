import { useCallback, useEffect, useRef, useState } from 'react';
import { getSeasonalPlan } from '../api/client';
import type {
  SeasonalPlan as SeasonalPlanType,
  PlannedActivity,
  ActivityStatus,
} from '../types';

const STATUS_COLORS: Record<ActivityStatus, string> = {
  Upcoming: '#3b82f6',
  Active: '#22c55e',
  Completed: '#6b7280',
  Missed: '#ef4444',
};

const STATUS_ICONS: Record<ActivityStatus, string> = {
  Upcoming: '○',
  Active: '●',
  Completed: '✓',
  Missed: '✗',
};

const CONFIDENCE_DOTS: Record<string, string> = {
  High: '●●●',
  Medium: '●●○',
  Low: '●○○',
};

function formatDate(dateStr: string): string {
  const d = new Date(dateStr + 'T12:00:00');
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

function formatDateRange(start: string, end: string): string {
  return `${formatDate(start)} – ${formatDate(end)}`;
}

function daysFromNow(dateStr: string): number {
  const d = new Date(dateStr + 'T12:00:00');
  const now = new Date();
  now.setHours(12, 0, 0, 0);
  return Math.round((d.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
}

function ActivityCard({ activity }: { activity: PlannedActivity }) {
  const [expanded, setExpanded] = useState(false);
  const statusColor = STATUS_COLORS[activity.status];
  const daysUntil = daysFromNow(activity.date_window.predicted_start);
  const daysUntilEnd = daysFromNow(activity.date_window.predicted_end);

  const timing =
    activity.status === 'Upcoming'
      ? daysUntil === 1
        ? 'Starts tomorrow'
        : `Starts in ${daysUntil} days`
      : activity.status === 'Active'
        ? daysUntilEnd <= 0
          ? 'Window closing today'
          : `${daysUntilEnd} days remaining`
        : activity.status === 'Missed'
          ? 'Window has passed'
          : 'Done';

  return (
    <div style={styles.card}>
      <div style={styles.cardHeader}>
        <div style={styles.timeline}>
          <div
            style={{
              ...styles.dot,
              backgroundColor: statusColor,
              boxShadow:
                activity.status === 'Active'
                  ? `0 0 8px ${statusColor}`
                  : 'none',
            }}
          />
          <div style={styles.line} />
        </div>
        <div style={styles.cardBody}>
          <div style={styles.cardTop}>
            <div>
              <span style={{ ...styles.statusBadge, color: statusColor }}>
                {STATUS_ICONS[activity.status]} {activity.status}
              </span>
              <span style={styles.category}>{activity.category}</span>
            </div>
            <span style={styles.dateRange}>
              {formatDateRange(
                activity.date_window.predicted_start,
                activity.date_window.predicted_end,
              )}
            </span>
          </div>
          <h3 style={styles.activityName}>{activity.name}</h3>
          <p style={styles.description}>{activity.description}</p>
          <div style={styles.metaRow}>
            <span style={styles.timing}>{timing}</span>
            <span style={styles.confidence}>
              {CONFIDENCE_DOTS[activity.date_window.confidence]}{' '}
              {activity.date_window.confidence}
            </span>
            <button
              onClick={() => setExpanded(!expanded)}
              style={styles.expandBtn}
            >
              {expanded ? 'Less' : 'Details'}
            </button>
          </div>
          {expanded && (
            <div style={styles.details}>
              {activity.details.soil_temp_trigger && (
                <div style={styles.detailRow}>
                  <span style={styles.detailLabel}>Soil Temp Trigger:</span>
                  <span>{activity.details.soil_temp_trigger}</span>
                </div>
              )}
              {activity.details.rate && (
                <div style={styles.detailRow}>
                  <span style={styles.detailLabel}>Rate:</span>
                  <span>{activity.details.rate}</span>
                </div>
              )}
              {activity.details.product_suggestions.length > 0 && (
                <div style={styles.detailRow}>
                  <span style={styles.detailLabel}>Products:</span>
                  <span>
                    {activity.details.product_suggestions.join(', ')}
                  </span>
                </div>
              )}
              {activity.details.notes && (
                <div style={styles.detailRow}>
                  <span style={styles.detailLabel}>Notes:</span>
                  <span>{activity.details.notes}</span>
                </div>
              )}
              {activity.date_window.earliest_historical && (
                <div style={styles.detailRow}>
                  <span style={styles.detailLabel}>Historical range:</span>
                  <span>
                    {formatDate(activity.date_window.earliest_historical)}
                    {activity.date_window.latest_historical &&
                      ` – ${formatDate(activity.date_window.latest_historical)}`}
                  </span>
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default function SeasonalPlan() {
  const [plan, setPlan] = useState<SeasonalPlanType | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [year, setYear] = useState(new Date().getFullYear());
  const abortRef = useRef<AbortController | null>(null);

  const fetchPlan = useCallback(async (y: number) => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    try {
      const result = await getSeasonalPlan(y);
      if (!controller.signal.aborted) {
        setPlan(result);
        setError(null);
      }
    } catch (e) {
      if (!controller.signal.aborted) {
        setError(e instanceof Error ? e.message : 'Failed to load');
      }
    } finally {
      if (!controller.signal.aborted) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    fetchPlan(year);
  }, [year, fetchPlan]);

  const currentYear = new Date().getFullYear();
  const yearOptions = [currentYear - 1, currentYear, currentYear + 1];

  if (loading) {
    return (
      <div style={styles.page}>
        <h1 style={styles.title}>Seasonal Plan</h1>
        <p style={styles.loadingText}>
          Analyzing historical soil data to build your seasonal plan...
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <div style={styles.page}>
        <h1 style={styles.title}>Seasonal Plan</h1>
        <div style={styles.error}>Failed to load seasonal plan: {error}</div>
      </div>
    );
  }

  if (!plan) return null;

  const statusCounts = plan.activities.reduce(
    (acc, a) => {
      acc[a.status] = (acc[a.status] || 0) + 1;
      return acc;
    },
    {} as Record<string, number>,
  );

  return (
    <div style={styles.page}>
      <div style={styles.header}>
        <div>
          <h1 style={styles.title}>Seasonal Plan</h1>
          <p style={styles.subtitle}>
            Predicted activity windows based on {plan.data_years_used} years of
            soil temperature data
          </p>
        </div>
        <div style={styles.yearSelector}>
          {yearOptions.map((y) => (
            <button
              key={y}
              onClick={() => setYear(y)}
              style={{
                ...styles.yearBtn,
                backgroundColor: y === year ? '#3182ce' : '#e2e8f0',
                color: y === year ? '#fff' : '#4a5568',
              }}
            >
              {y}
            </button>
          ))}
        </div>
      </div>

      <div style={styles.statusSummary}>
        {(['Active', 'Upcoming', 'Completed', 'Missed'] as ActivityStatus[]).map(
          (s) =>
            statusCounts[s] ? (
              <span
                key={s}
                style={{
                  ...styles.summaryBadge,
                  backgroundColor: STATUS_COLORS[s] + '18',
                  color: STATUS_COLORS[s],
                  borderColor: STATUS_COLORS[s] + '40',
                }}
              >
                {STATUS_ICONS[s]} {statusCounts[s]} {s}
              </span>
            ) : null,
        )}
      </div>

      <div style={styles.timeline}>
        {plan.activities.map((activity) => (
          <ActivityCard key={activity.id} activity={activity} />
        ))}
      </div>

      {plan.activities.length === 0 && (
        <div style={styles.empty}>
          No activities could be projected for {year}. Insufficient historical
          data may be the cause.
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  page: {
    maxWidth: 800,
    margin: '0 auto',
  },
  header: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'flex-start',
    marginBottom: '1.5rem',
    flexWrap: 'wrap',
    gap: '1rem',
  },
  title: {
    fontSize: '1.5rem',
    fontWeight: 700,
    color: '#1a202c',
    margin: 0,
  },
  subtitle: {
    color: '#718096',
    fontSize: '0.85rem',
    margin: '0.25rem 0 0',
  },
  yearSelector: {
    display: 'flex',
    gap: 6,
  },
  yearBtn: {
    padding: '0.4rem 0.9rem',
    border: 'none',
    borderRadius: 6,
    cursor: 'pointer',
    fontSize: '0.85rem',
    fontWeight: 600,
    transition: 'all 0.15s',
  },
  statusSummary: {
    display: 'flex',
    gap: 8,
    marginBottom: '1.5rem',
    flexWrap: 'wrap' as const,
  },
  summaryBadge: {
    padding: '0.3rem 0.7rem',
    borderRadius: 20,
    fontSize: '0.8rem',
    fontWeight: 600,
    border: '1px solid',
  },
  card: {
    marginBottom: 0,
  },
  cardHeader: {
    display: 'flex',
    gap: 0,
  },
  timeline: {
    display: 'flex',
    flexDirection: 'column' as const,
  },
  dot: {
    width: 14,
    height: 14,
    borderRadius: '50%',
    flexShrink: 0,
    marginTop: 4,
  },
  line: {
    width: 2,
    flex: 1,
    backgroundColor: '#e2e8f0',
    margin: '4px auto 0',
    minHeight: 20,
  },
  cardBody: {
    flex: 1,
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '1rem 1.2rem',
    marginLeft: 12,
    marginBottom: 12,
    border: '1px solid #e2e8f0',
  },
  cardTop: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 4,
    flexWrap: 'wrap' as const,
    gap: 4,
  },
  statusBadge: {
    fontSize: '0.75rem',
    fontWeight: 700,
    textTransform: 'uppercase' as const,
    letterSpacing: '0.03em',
  },
  category: {
    fontSize: '0.75rem',
    color: '#a0aec0',
    marginLeft: 10,
  },
  dateRange: {
    fontSize: '0.8rem',
    color: '#718096',
    fontWeight: 500,
  },
  activityName: {
    fontSize: '1.05rem',
    fontWeight: 600,
    color: '#2d3748',
    margin: '0.3rem 0',
  },
  description: {
    fontSize: '0.85rem',
    color: '#4a5568',
    margin: '0 0 0.5rem',
    lineHeight: 1.5,
  },
  metaRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 12,
    fontSize: '0.8rem',
  },
  timing: {
    color: '#718096',
    fontWeight: 500,
  },
  confidence: {
    color: '#a0aec0',
    fontSize: '0.75rem',
  },
  expandBtn: {
    background: 'none',
    border: 'none',
    color: '#3182ce',
    cursor: 'pointer',
    fontSize: '0.8rem',
    fontWeight: 500,
    padding: 0,
    marginLeft: 'auto',
  },
  details: {
    marginTop: '0.75rem',
    paddingTop: '0.75rem',
    borderTop: '1px solid #edf2f7',
  },
  detailRow: {
    display: 'flex',
    gap: 8,
    marginBottom: 6,
    fontSize: '0.82rem',
    color: '#4a5568',
    flexWrap: 'wrap' as const,
  },
  detailLabel: {
    fontWeight: 600,
    color: '#2d3748',
    whiteSpace: 'nowrap' as const,
  },
  loadingText: {
    color: '#718096',
    marginTop: '2rem',
  },
  error: {
    color: '#e53e3e',
    backgroundColor: '#fff5f5',
    padding: '1rem',
    borderRadius: 8,
    border: '1px solid #fed7d7',
    marginTop: '1rem',
  },
  empty: {
    color: '#718096',
    textAlign: 'center' as const,
    padding: '2rem',
    backgroundColor: '#fff',
    borderRadius: 8,
    border: '1px solid #e2e8f0',
  },
};
