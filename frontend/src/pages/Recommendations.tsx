import { useCallback, useEffect, useState } from 'react';
import {
  getRecommendations,
  patchRecommendation,
} from '../api/client';
import type { Recommendation } from '../types';
import { SEVERITY_COLORS, SEVERITY_SYMBOLS } from '../types';

export default function Recommendations() {
  const [recs, setRecs] = useState<Recommendation[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const fetchRecs = useCallback(async () => {
    try {
      const data = await getRecommendations();
      setRecs(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchRecs();
  }, [fetchRecs]);

  const handleAction = async (
    id: string,
    action: 'addressed' | 'dismissed'
  ) => {
    try {
      await patchRecommendation(id, { [action]: true });
      setRecs((prev) => prev.filter((r) => r.id !== id));
      if (selected === id) setSelected(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update');
    }
  };

  const selectedRec = recs.find((r) => r.id === selected);

  if (loading) return <div style={{ color: '#718096', padding: '2rem' }}>Loading...</div>;

  return (
    <div>
      <h1 style={styles.title}>Recommendations</h1>

      {error && <div style={styles.error}>{error}</div>}

      {recs.length === 0 ? (
        <div style={styles.empty}>
          No active recommendations. Your lawn is looking good!
        </div>
      ) : (
        <div style={styles.splitView}>
          {/* List */}
          <div style={styles.list}>
            {recs.map((rec) => {
              const color = SEVERITY_COLORS[rec.severity];
              const symbol = SEVERITY_SYMBOLS[rec.severity];
              const isSelected = rec.id === selected;

              return (
                <div
                  key={rec.id}
                  style={{
                    ...styles.listItem,
                    borderLeftColor: color,
                    backgroundColor: isSelected ? '#ebf8ff' : '#fff',
                  }}
                  onClick={() => setSelected(rec.id)}
                >
                  <div style={styles.listHeader}>
                    <span style={{ ...styles.badge, backgroundColor: color }}>
                      {symbol} {rec.severity}
                    </span>
                    <span style={styles.category}>{rec.category}</span>
                  </div>
                  <div style={styles.listTitle}>{rec.title}</div>
                  <div style={styles.listDesc}>{rec.description}</div>
                  <div style={styles.actions}>
                    <button
                      style={styles.addressBtn}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleAction(rec.id, 'addressed');
                      }}
                    >
                      Mark Addressed
                    </button>
                    <button
                      style={styles.dismissBtn}
                      onClick={(e) => {
                        e.stopPropagation();
                        handleAction(rec.id, 'dismissed');
                      }}
                    >
                      Dismiss
                    </button>
                  </div>
                </div>
              );
            })}
          </div>

          {/* Detail */}
          {selectedRec && (
            <div style={styles.detail}>
              <h2 style={styles.detailTitle}>{selectedRec.title}</h2>
              <span
                style={{
                  ...styles.badge,
                  backgroundColor: SEVERITY_COLORS[selectedRec.severity],
                  marginBottom: 8,
                  display: 'inline-block',
                }}
              >
                {SEVERITY_SYMBOLS[selectedRec.severity]} {selectedRec.severity}
              </span>
              <span style={{ ...styles.category, marginLeft: 8 }}>
                {selectedRec.category}
              </span>

              <p style={styles.detailDesc}>{selectedRec.description}</p>

              {selectedRec.explanation && (
                <div style={styles.section}>
                  <h3 style={styles.sectionTitle}>Explanation</h3>
                  <p style={styles.sectionText}>{selectedRec.explanation}</p>
                </div>
              )}

              {selectedRec.data_points.length > 0 && (
                <div style={styles.section}>
                  <h3 style={styles.sectionTitle}>Data Points</h3>
                  <table style={styles.dataTable}>
                    <tbody>
                      {selectedRec.data_points.map((dp, i) => (
                        <tr key={i}>
                          <td style={styles.dpLabel}>{dp.label}</td>
                          <td style={styles.dpValue}>{dp.value}</td>
                          <td style={styles.dpSource}>{dp.source}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}

              {selectedRec.suggested_action && (
                <div style={styles.section}>
                  <h3 style={styles.sectionTitle}>Suggested Action</h3>
                  <p style={styles.sectionText}>
                    {selectedRec.suggested_action}
                  </p>
                </div>
              )}
            </div>
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
  empty: {
    color: '#48bb78',
    fontSize: '1rem',
    padding: '2rem',
    backgroundColor: '#fff',
    borderRadius: 8,
    textAlign: 'center' as const,
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  splitView: { display: 'flex', gap: '1.5rem', alignItems: 'flex-start' },
  list: { flex: 1, minWidth: 0 },
  listItem: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '0.8rem 1rem',
    marginBottom: '0.5rem',
    borderLeft: '4px solid',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
    cursor: 'pointer',
  },
  listHeader: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    marginBottom: 4,
  },
  badge: {
    color: '#fff',
    fontSize: '0.7rem',
    fontWeight: 600,
    padding: '2px 8px',
    borderRadius: 10,
  },
  category: { fontSize: '0.75rem', color: '#718096' },
  listTitle: {
    fontWeight: 600,
    fontSize: '0.9rem',
    color: '#2d3748',
    marginBottom: 2,
  },
  listDesc: { fontSize: '0.82rem', color: '#4a5568', lineHeight: 1.4 },
  actions: { display: 'flex', gap: 8, marginTop: 8 },
  addressBtn: {
    padding: '4px 12px',
    backgroundColor: '#48bb78',
    color: '#fff',
    border: 'none',
    borderRadius: 4,
    cursor: 'pointer',
    fontSize: '0.75rem',
    fontWeight: 600,
  },
  dismissBtn: {
    padding: '4px 12px',
    backgroundColor: 'transparent',
    color: '#718096',
    border: '1px solid #e2e8f0',
    borderRadius: 4,
    cursor: 'pointer',
    fontSize: '0.75rem',
  },
  detail: {
    flex: 1,
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '1.2rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
    position: 'sticky' as const,
    top: 16,
  },
  detailTitle: { margin: '0 0 8px', fontSize: '1.1rem', color: '#1a202c' },
  detailDesc: { color: '#4a5568', fontSize: '0.9rem', lineHeight: 1.5 },
  section: { marginTop: '1rem' },
  sectionTitle: {
    fontSize: '0.85rem',
    fontWeight: 600,
    color: '#2d3748',
    margin: '0 0 4px',
  },
  sectionText: { color: '#4a5568', fontSize: '0.85rem', lineHeight: 1.5, margin: 0 },
  dataTable: { width: '100%', fontSize: '0.82rem' },
  dpLabel: { padding: '4px 8px 4px 0', color: '#718096', fontWeight: 500 },
  dpValue: { padding: '4px 8px', color: '#2d3748', fontWeight: 600 },
  dpSource: { padding: '4px 0 4px 8px', color: '#a0aec0', fontSize: '0.75rem' },
};
