import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  createApplication,
  deleteApplication,
  getApplications,
  listPlants,
} from '../api/client';
import { appTypeBadgeStyle, sharedStyles } from '../styles/shared';
import type { Application, ApplicationType, Plant } from '../types';
import {
  APPLICATION_TYPE_LABELS,
  canTargetPlant,
  isPlantRequiredApplicationType,
  isTurfOnlyApplicationType,
} from '../types';

type ScopeFilter = 'all' | 'turf' | 'landscape';

function addDaysISO(dateStr: string, days: number): string {
  const d = new Date(dateStr + 'T00:00:00');
  d.setDate(d.getDate() + days);
  return d.toISOString().split('T')[0];
}

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
  'Pruning',
  'PlantFertilizer',
  'Mulching',
  'Deadheading',
  'WinterProtection',
];

export default function Applications() {
  const [apps, setApps] = useState<Application[]>([]);
  const [filter, setFilter] = useState('');
  const [scopeFilter, setScopeFilter] = useState<ScopeFilter>('all');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [plants, setPlants] = useState<Plant[]>([]);

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

  useEffect(() => {
    listPlants()
      .then(setPlants)
      .catch(() => setPlants([]));
  }, []);

  const plantNameById = useMemo(() => {
    const map = new Map<number, string>();
    for (const p of plants) {
      if (p.id != null) map.set(p.id, p.common_name);
    }
    return map;
  }, [plants]);

  const visibleApps = useMemo(() => {
    if (scopeFilter === 'all') return apps;
    if (scopeFilter === 'landscape') return apps.filter((a) => a.plant_id != null);
    return apps.filter((a) => a.plant_id == null);
  }, [apps, scopeFilter]);

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

      {showForm && (
        <AddForm
          plants={plants}
          onCreated={handleCreated}
          onError={setError}
        />
      )}

      {error && <div style={sharedStyles.error}>{error}</div>}

      {/* Filters */}
      <div style={styles.filterRow}>
        <label style={styles.filterLabel}>Scope:</label>
        <div style={styles.scopeToggle}>
          {(['all', 'turf', 'landscape'] as ScopeFilter[]).map((s) => (
            <button
              key={s}
              type="button"
              style={{
                ...styles.scopeBtn,
                ...(scopeFilter === s ? styles.scopeBtnActive : {}),
              }}
              onClick={() => setScopeFilter(s)}
            >
              {s === 'all' ? 'All' : s === 'turf' ? 'Turf' : 'Landscape'}
            </button>
          ))}
        </div>

        <label style={{ ...styles.filterLabel, marginLeft: 16 }}>
          Type:
        </label>
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
      ) : visibleApps.length === 0 ? (
        <p style={sharedStyles.empty}>No applications found.</p>
      ) : (
        <table style={sharedStyles.table}>
          <thead>
            <tr>
              <th style={sharedStyles.th}>Date</th>
              <th style={sharedStyles.th}>Type</th>
              <th style={sharedStyles.th}>Target</th>
              <th style={sharedStyles.th}>Product</th>
              <th style={sharedStyles.th}>Rate/1k sqft</th>
              <th style={sharedStyles.th}>N-P-K</th>
              <th style={sharedStyles.th}>Coverage</th>
              <th style={sharedStyles.th}>Follow-up</th>
              <th style={sharedStyles.th}>Notes</th>
              <th style={sharedStyles.th}></th>
            </tr>
          </thead>
          <tbody>
            {visibleApps.map((app, index) => (
              <tr key={app.id ?? `app-${index}`}>
                <td style={sharedStyles.td}>{app.application_date}</td>
                <td style={sharedStyles.td}>
                  <span
                    style={appTypeBadgeStyle(sharedStyles.badge, app.application_type)}
                  >
                    {APPLICATION_TYPE_LABELS[app.application_type]}
                  </span>
                </td>
                <td style={sharedStyles.td}>
                  {app.plant_id != null
                    ? (plantNameById.get(app.plant_id) ?? `Plant #${app.plant_id}`)
                    : <span style={styles.turfTag}>Turf</span>}
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
                <td style={sharedStyles.td}>
                  {app.follow_up_date ?? '-'}
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
  plants,
  onCreated,
  onError,
}: {
  plants: Plant[];
  onCreated: () => void;
  onError: (msg: string) => void;
}) {
  const [appType, setAppType] = useState<ApplicationType>('Fertilizer');
  const [productName, setProductName] = useState('');
  const [date, setDate] = useState(new Date().toISOString().split('T')[0]);
  const [rate, setRate] = useState('');
  const [coverage, setCoverage] = useState('');
  const [notes, setNotes] = useState('');
  const [nitrogenPct, setNitrogenPct] = useState('');
  const [phosphorusPct, setPhosphorusPct] = useState('');
  const [potassiumPct, setPotassiumPct] = useState('');
  const [plantId, setPlantId] = useState<string>('');
  const [followUpEnabled, setFollowUpEnabled] = useState(false);
  const [followUpDate, setFollowUpDate] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const plantRequired = isPlantRequiredApplicationType(appType);
  const turfOnly = isTurfOnlyApplicationType(appType);
  const plantSelectable = canTargetPlant(appType);

  // When type changes, drop any incompatible plant selection.
  useEffect(() => {
    if (turfOnly) setPlantId('');
  }, [turfOnly]);

  const setFollowUpOffset = (days: number) => {
    setFollowUpEnabled(true);
    setFollowUpDate(addDaysISO(date, days));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (plantRequired && !plantId) {
      onError('Select which plant this action is for.');
      return;
    }
    if (followUpEnabled && !followUpDate) {
      onError('Pick a follow-up date or uncheck the follow-up option.');
      return;
    }
    if (followUpEnabled && followUpDate < date) {
      onError('Follow-up date must be on or after the application date.');
      return;
    }
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
        plant_id: plantSelectable && plantId ? parseInt(plantId, 10) : undefined,
        follow_up_date: followUpEnabled && followUpDate ? followUpDate : undefined,
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
            onChange={(e) => setAppType(e.target.value as ApplicationType)}
          >
            {APP_TYPES.map((t) => (
              <option key={t} value={t}>
                {APPLICATION_TYPE_LABELS[t]}
              </option>
            ))}
          </select>
        </div>
        {plantSelectable && (
          <div>
            <label style={styles.formLabel}>
              Plant {plantRequired ? '' : '(optional — leave blank for turf)'}
            </label>
            <select
              style={styles.input}
              value={plantId}
              onChange={(e) => setPlantId(e.target.value)}
              required={plantRequired}
            >
              <option value="">
                {plantRequired ? 'Select a plant…' : 'Turf (no specific plant)'}
              </option>
              {plants.map((p) => (
                <option key={p.id ?? p.common_name} value={p.id ?? ''}>
                  {p.common_name}
                  {p.location ? ` (${p.location})` : ''}
                </option>
              ))}
            </select>
          </div>
        )}
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

      {/* Follow-up scheduling */}
      <div style={styles.followUpRow}>
        <label style={styles.followUpLabel}>
          <input
            type="checkbox"
            checked={followUpEnabled}
            onChange={(e) => {
              setFollowUpEnabled(e.target.checked);
              if (!e.target.checked) setFollowUpDate('');
            }}
          />
          {' '}Schedule a follow-up
        </label>
        {followUpEnabled && (
          <>
            <input
              type="date"
              style={styles.followUpInput}
              value={followUpDate}
              min={date}
              onChange={(e) => setFollowUpDate(e.target.value)}
            />
            <div style={styles.shortcutRow}>
              {[
                { label: '+2 wk', days: 14 },
                { label: '+4 wk', days: 28 },
                { label: '+6 wk', days: 42 },
              ].map((s) => (
                <button
                  key={s.label}
                  type="button"
                  style={styles.shortcutBtn}
                  onClick={() => setFollowUpOffset(s.days)}
                >
                  {s.label}
                </button>
              ))}
            </div>
          </>
        )}
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
  scopeToggle: {
    display: 'inline-flex',
    border: '1px solid #e2e8f0',
    borderRadius: 6,
    overflow: 'hidden',
  },
  scopeBtn: {
    padding: '0.4rem 0.8rem',
    backgroundColor: '#fff',
    border: 'none',
    cursor: 'pointer',
    fontSize: '0.8rem',
    color: '#4a5568',
    borderRight: '1px solid #e2e8f0',
  },
  scopeBtnActive: {
    backgroundColor: '#3182ce',
    color: '#fff',
  },
  turfTag: {
    display: 'inline-block',
    padding: '1px 8px',
    borderRadius: 10,
    fontSize: '0.7rem',
    color: '#4a5568',
    backgroundColor: '#edf2f7',
  },
  followUpRow: {
    display: 'flex',
    flexWrap: 'wrap' as const,
    alignItems: 'center',
    gap: 12,
    padding: '0.5rem 0',
    marginBottom: '0.75rem',
    borderTop: '1px dashed #e2e8f0',
  },
  followUpLabel: {
    fontSize: '0.85rem',
    color: '#4a5568',
    fontWeight: 600,
    cursor: 'pointer',
  },
  followUpInput: {
    padding: '0.4rem 0.6rem',
    borderRadius: 6,
    border: '1px solid #e2e8f0',
    fontSize: '0.85rem',
  },
  shortcutRow: {
    display: 'flex',
    gap: 6,
  },
  shortcutBtn: {
    padding: '0.3rem 0.6rem',
    borderRadius: 4,
    border: '1px solid #cbd5e0',
    backgroundColor: '#f7fafc',
    cursor: 'pointer',
    fontSize: '0.75rem',
    color: '#4a5568',
  },
};
