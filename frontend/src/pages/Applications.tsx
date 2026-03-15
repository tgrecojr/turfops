import { useCallback, useEffect, useState } from 'react';
import {
  createApplication,
  deleteApplication,
  getApplications,
} from '../api/client';
import { appTypeBadgeStyle, sharedStyles } from '../styles/shared';
import type { Application, ApplicationType } from '../types';
import { APPLICATION_TYPE_LABELS } from '../types';

const APP_TYPES: ApplicationType[] = [
  'PreEmergent',
  'PostEmergent',
  'Fertilizer',
  'Fungicide',
  'Insecticide',
  'GrubControl',
  'Overseed',
  'Aeration',
  'Dethatching',
  'Lime',
  'Sulfur',
  'Wetting',
  'Mowing',
  'Other',
];

export default function Applications() {
  const [apps, setApps] = useState<Application[]>([]);
  const [filter, setFilter] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [deletingId, setDeletingId] = useState<number | null>(null);

  const fetchApps = useCallback(async () => {
    try {
      const data = await getApplications(filter || undefined);
      setApps(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load');
    } finally {
      setLoading(false);
    }
  }, [filter]);

  useEffect(() => {
    fetchApps();
  }, [fetchApps]);

  const handleDelete = async (id: number) => {
    if (!confirm('Delete this application?')) return;
    setDeletingId(id);
    try {
      await deleteApplication(id);
      setApps((prev) => prev.filter((a) => a.id !== id));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete');
    } finally {
      setDeletingId(null);
    }
  };

  const handleCreated = () => {
    setShowForm(false);
    fetchApps();
  };

  return (
    <div>
      <div style={sharedStyles.headerRow}>
        <h1 style={sharedStyles.pageTitle}>Applications</h1>
        <button style={styles.addBtn} onClick={() => setShowForm(!showForm)}>
          {showForm ? 'Cancel' : '+ Add Application'}
        </button>
      </div>

      {showForm && <AddForm onCreated={handleCreated} onError={setError} />}

      {error && <div style={sharedStyles.error}>{error}</div>}

      {/* Filter */}
      <div style={styles.filterRow}>
        <label style={styles.filterLabel}>Filter by type:</label>
        <select
          style={styles.select}
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
        >
          <option value="">All</option>
          {APP_TYPES.map((t) => (
            <option key={t} value={t}>
              {APPLICATION_TYPE_LABELS[t]}
            </option>
          ))}
        </select>
      </div>

      {loading ? (
        <p style={sharedStyles.loading}>Loading...</p>
      ) : apps.length === 0 ? (
        <p style={sharedStyles.empty}>No applications found.</p>
      ) : (
        <table style={sharedStyles.table}>
          <thead>
            <tr>
              <th style={sharedStyles.th}>Date</th>
              <th style={sharedStyles.th}>Type</th>
              <th style={sharedStyles.th}>Product</th>
              <th style={sharedStyles.th}>Rate/1k sqft</th>
              <th style={sharedStyles.th}>N-P-K</th>
              <th style={sharedStyles.th}>Coverage</th>
              <th style={sharedStyles.th}>Notes</th>
              <th style={sharedStyles.th}></th>
            </tr>
          </thead>
          <tbody>
            {apps.map((app, index) => (
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
                <td style={sharedStyles.td}>
                  {app.rate_per_1000sqft != null
                    ? app.rate_per_1000sqft.toFixed(2)
                    : '-'}
                </td>
                <td style={sharedStyles.td}>
                  {app.nitrogen_pct != null
                    ? `${app.nitrogen_pct}-${app.phosphorus_pct ?? 0}-${app.potassium_pct ?? 0}`
                    : '-'}
                </td>
                <td style={sharedStyles.td}>
                  {app.coverage_sqft != null
                    ? `${app.coverage_sqft.toLocaleString()} sqft`
                    : '-'}
                </td>
                <td style={sharedStyles.td}>{app.notes || '-'}</td>
                <td style={sharedStyles.td}>
                  <button
                    style={styles.deleteBtn}
                    onClick={() => app.id != null && handleDelete(app.id)}
                    disabled={deletingId === app.id}
                  >
                    {deletingId === app.id ? 'Deleting...' : 'Delete'}
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function AddForm({
  onCreated,
  onError,
}: {
  onCreated: () => void;
  onError: (msg: string) => void;
}) {
  const [appType, setAppType] = useState<string>('Fertilizer');
  const [productName, setProductName] = useState('');
  const [date, setDate] = useState(new Date().toISOString().split('T')[0]);
  const [rate, setRate] = useState('');
  const [coverage, setCoverage] = useState('');
  const [notes, setNotes] = useState('');
  const [nitrogenPct, setNitrogenPct] = useState('');
  const [phosphorusPct, setPhosphorusPct] = useState('');
  const [potassiumPct, setPotassiumPct] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      await createApplication({
        application_type: appType,
        product_name: productName || undefined,
        application_date: date,
        rate_per_1000sqft: rate ? parseFloat(rate) : undefined,
        coverage_sqft: coverage ? parseFloat(coverage) : undefined,
        notes: notes || undefined,
        nitrogen_pct: nitrogenPct ? parseFloat(nitrogenPct) : undefined,
        phosphorus_pct: phosphorusPct ? parseFloat(phosphorusPct) : undefined,
        potassium_pct: potassiumPct ? parseFloat(potassiumPct) : undefined,
      });
      onCreated();
    } catch (err) {
      onError(err instanceof Error ? err.message : 'Failed to create');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} style={styles.form}>
      <div style={styles.formGrid}>
        <div>
          <label style={styles.formLabel}>Type</label>
          <select
            style={styles.input}
            value={appType}
            onChange={(e) => setAppType(e.target.value)}
          >
            {APP_TYPES.map((t) => (
              <option key={t} value={t}>
                {APPLICATION_TYPE_LABELS[t]}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label style={styles.formLabel}>Date</label>
          <input
            type="date"
            style={styles.input}
            value={date}
            onChange={(e) => setDate(e.target.value)}
            required
          />
        </div>
        <div>
          <label style={styles.formLabel}>Product Name</label>
          <input
            style={styles.input}
            value={productName}
            onChange={(e) => setProductName(e.target.value)}
            placeholder="e.g. Milorganite"
          />
        </div>
        <div>
          <label style={styles.formLabel}>Rate / 1k sqft</label>
          <input
            type="number"
            step="0.01"
            style={styles.input}
            value={rate}
            onChange={(e) => setRate(e.target.value)}
            placeholder="lbs"
          />
        </div>
        <div>
          <label style={styles.formLabel}>Coverage (sqft)</label>
          <input
            type="number"
            style={styles.input}
            value={coverage}
            onChange={(e) => setCoverage(e.target.value)}
          />
        </div>
        <div>
          <label style={styles.formLabel}>Notes</label>
          <input
            style={styles.input}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
          />
        </div>
        <div>
          <label style={styles.formLabel}>N %</label>
          <input
            type="number"
            step="0.1"
            style={styles.input}
            value={nitrogenPct}
            onChange={(e) => setNitrogenPct(e.target.value)}
            placeholder="e.g. 29"
          />
        </div>
        <div>
          <label style={styles.formLabel}>P %</label>
          <input
            type="number"
            step="0.1"
            style={styles.input}
            value={phosphorusPct}
            onChange={(e) => setPhosphorusPct(e.target.value)}
            placeholder="e.g. 0"
          />
        </div>
        <div>
          <label style={styles.formLabel}>K %</label>
          <input
            type="number"
            step="0.1"
            style={styles.input}
            value={potassiumPct}
            onChange={(e) => setPotassiumPct(e.target.value)}
            placeholder="e.g. 4"
          />
        </div>
      </div>
      <button type="submit" style={styles.submitBtn} disabled={submitting}>
        {submitting ? 'Saving...' : 'Save Application'}
      </button>
    </form>
  );
}

const styles: Record<string, React.CSSProperties> = {
  addBtn: {
    padding: '0.5rem 1rem',
    backgroundColor: '#48bb78',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    cursor: 'pointer',
    fontWeight: 600,
    fontSize: '0.85rem',
  },
  filterRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 8,
    marginBottom: '1rem',
  },
  filterLabel: { fontSize: '0.85rem', color: '#4a5568' },
  select: {
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid #e2e8f0',
    fontSize: '0.85rem',
  },
  deleteBtn: {
    padding: '4px 10px',
    backgroundColor: 'transparent',
    color: '#e53e3e',
    border: '1px solid #e53e3e',
    borderRadius: 4,
    cursor: 'pointer',
    fontSize: '0.75rem',
  },
  form: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '1rem',
    marginBottom: '1rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
  },
  formGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
    gap: '0.75rem',
    marginBottom: '0.75rem',
  },
  formLabel: {
    display: 'block',
    fontSize: '0.75rem',
    color: '#718096',
    marginBottom: 4,
    fontWeight: 600,
  },
  input: {
    width: '100%',
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid #e2e8f0',
    fontSize: '0.85rem',
  },
  submitBtn: {
    padding: '0.5rem 1.5rem',
    backgroundColor: '#3182ce',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    cursor: 'pointer',
    fontWeight: 600,
    fontSize: '0.85rem',
  },
};
