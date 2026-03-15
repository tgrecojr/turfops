import { useEffect, useState } from 'react';
import { getCalendar } from '../api/client';
import { appTypeBadgeStyle } from '../styles/shared';
import type { Application, CalendarResponse } from '../types';
import { APPLICATION_TYPE_COLORS, APPLICATION_TYPE_LABELS } from '../types';

export default function Calendar() {
  const today = new Date();
  const [year, setYear] = useState(today.getFullYear());
  const [month, setMonth] = useState(today.getMonth() + 1);
  const [data, setData] = useState<CalendarResponse | null>(null);
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getCalendar(year, month)
      .then((d) => {
        if (!cancelled) {
          setData(d);
          setError(null);
        }
      })
      .catch((e) => {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : 'Failed to load');
        }
      });
    return () => { cancelled = true; };
  }, [year, month]);

  const prevMonth = () => {
    if (month === 1) {
      setMonth(12);
      setYear(year - 1);
    } else {
      setMonth(month - 1);
    }
    setSelectedDate(null);
  };

  const nextMonth = () => {
    if (month === 12) {
      setMonth(1);
      setYear(year + 1);
    } else {
      setMonth(month + 1);
    }
    setSelectedDate(null);
  };

  const monthName = new Date(year, month - 1).toLocaleString('default', {
    month: 'long',
  });

  // Build calendar grid
  const firstDay = new Date(year, month - 1, 1).getDay();
  const daysInMonth = new Date(year, month, 0).getDate();
  const weeks: (number | null)[][] = [];
  let week: (number | null)[] = Array(firstDay).fill(null);

  for (let d = 1; d <= daysInMonth; d++) {
    week.push(d);
    if (week.length === 7) {
      weeks.push(week);
      week = [];
    }
  }
  if (week.length > 0) {
    while (week.length < 7) week.push(null);
    weeks.push(week);
  }

  const dateKey = (day: number) =>
    `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;

  const selectedApps: Application[] =
    selectedDate && data?.days[selectedDate] ? data.days[selectedDate] : [];

  return (
    <div>
      <h1 style={styles.title}>Calendar</h1>

      {error && <div style={styles.error}>{error}</div>}

      <div style={styles.navRow}>
        <button style={styles.navBtn} onClick={prevMonth}>
          &larr;
        </button>
        <span style={styles.monthLabel}>
          {monthName} {year}
        </span>
        <button style={styles.navBtn} onClick={nextMonth}>
          &rarr;
        </button>
      </div>

      <div style={styles.calGrid}>
        <table style={styles.table}>
          <thead>
            <tr>
              {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map((d) => (
                <th key={d} style={styles.dayHeader}>
                  {d}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {weeks.map((wk, wi) => (
              <tr key={wi}>
                {wk.map((day, di) => {
                  if (day === null)
                    return <td key={di} style={styles.emptyCell} />;
                  const dk = dateKey(day);
                  const dayApps = data?.days[dk] || [];
                  const isSelected = dk === selectedDate;
                  return (
                    <td
                      key={di}
                      style={{
                        ...styles.cell,
                        backgroundColor: isSelected ? '#ebf8ff' : '#fff',
                        cursor: dayApps.length > 0 ? 'pointer' : 'default',
                      }}
                      onClick={() =>
                        dayApps.length > 0 && setSelectedDate(dk)
                      }
                    >
                      <div style={styles.dayNum}>{day}</div>
                      <div style={styles.dots}>
                        {dayApps.map((a, i) => (
                          <span
                            key={i}
                            style={{
                              ...styles.dot,
                              backgroundColor:
                                APPLICATION_TYPE_COLORS[a.application_type],
                            }}
                            title={APPLICATION_TYPE_LABELS[a.application_type]}
                          />
                        ))}
                      </div>
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Detail panel */}
      {selectedDate && (
        <div style={styles.detail}>
          <h3 style={{ margin: '0 0 0.5rem' }}>{selectedDate}</h3>
          {selectedApps.length === 0 ? (
            <p style={{ color: '#a0aec0' }}>No applications on this date.</p>
          ) : (
            selectedApps.map((app) => (
              <div key={app.id} style={styles.detailCard}>
                <span
                  style={appTypeBadgeStyle(styles.badge, app.application_type)}
                >
                  {APPLICATION_TYPE_LABELS[app.application_type]}
                </span>
                {app.product_name && (
                  <span style={{ marginLeft: 8 }}>{app.product_name}</span>
                )}
                {app.notes && (
                  <div style={styles.notes}>{app.notes}</div>
                )}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  title: { margin: '0 0 1rem', fontSize: '1.5rem', color: '#1a202c' },
  error: {
    padding: '0.5rem 1rem',
    backgroundColor: '#fed7d7',
    color: '#c53030',
    borderRadius: 6,
    marginBottom: '1rem',
    fontSize: '0.85rem',
  },
  navRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 16,
    marginBottom: '1rem',
  },
  navBtn: {
    padding: '0.3rem 0.8rem',
    border: '1px solid #e2e8f0',
    borderRadius: 6,
    backgroundColor: '#fff',
    cursor: 'pointer',
    fontSize: '1rem',
  },
  monthLabel: { fontSize: '1.1rem', fontWeight: 600, color: '#2d3748' },
  calGrid: { marginBottom: '1rem' },
  table: {
    width: '100%',
    borderCollapse: 'collapse' as const,
    backgroundColor: '#fff',
    borderRadius: 8,
    overflow: 'hidden',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
    tableLayout: 'fixed' as const,
  },
  dayHeader: {
    padding: '0.5rem',
    fontSize: '0.75rem',
    color: '#718096',
    fontWeight: 600,
    textAlign: 'center' as const,
    borderBottom: '1px solid #e2e8f0',
  },
  cell: {
    padding: '0.5rem',
    height: 70,
    verticalAlign: 'top' as const,
    borderBottom: '1px solid #edf2f7',
    borderRight: '1px solid #edf2f7',
  },
  emptyCell: {
    backgroundColor: '#f7fafc',
    borderBottom: '1px solid #edf2f7',
    borderRight: '1px solid #edf2f7',
  },
  dayNum: { fontSize: '0.8rem', color: '#4a5568', marginBottom: 4 },
  dots: { display: 'flex', gap: 3, flexWrap: 'wrap' as const },
  dot: {
    width: 8,
    height: 8,
    borderRadius: '50%',
    display: 'inline-block',
  },
  detail: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '1rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  detailCard: {
    padding: '0.5rem 0',
    borderBottom: '1px solid #edf2f7',
    fontSize: '0.85rem',
  },
  badge: {
    display: 'inline-block',
    padding: '2px 8px',
    borderRadius: 10,
    fontSize: '0.75rem',
    fontWeight: 600,
    border: '1px solid',
  },
  notes: { color: '#718096', fontSize: '0.8rem', marginTop: 4 },
};
