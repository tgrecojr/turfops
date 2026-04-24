import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  createPlant,
  deletePlant,
  listPlants,
  refreshPlantPlan,
} from '../api/client';
import { sharedStyles } from '../styles/shared';
import type { MaintenanceTask, Plant, PlantType } from '../types';
import {
  IDENTIFICATION_CONFIDENCE_COLORS,
  PLANT_TYPE_LABELS,
  TASK_TYPE_LABELS,
} from '../types';

const PLANT_TYPES: PlantType[] = [
  'Shrub',
  'Tree',
  'Perennial',
  'Annual',
  'Vine',
  'Groundcover',
  'Other',
];

export default function Landscape() {
  const [plants, setPlants] = useState<Plant[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [refreshingId, setRefreshingId] = useState<number | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());

  const fetchPlants = useCallback(async () => {
    try {
      const data = await listPlants();
      setPlants(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load plants');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchPlants();
  }, [fetchPlants]);

  const sortedPlants = useMemo(
    () =>
      [...plants].sort((a, b) =>
        a.common_name.localeCompare(b.common_name, undefined, {
          sensitivity: 'base',
        })
      ),
    [plants]
  );

  const toggleExpanded = (id: number) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const handleCreated = (plant: Plant) => {
    setShowForm(false);
    setPlants((prev) => [...prev, plant]);
    if (plant.id != null) {
      const newId = plant.id;
      setExpandedIds((prev) => {
        const next = new Set(prev);
        next.add(newId);
        return next;
      });
    }
  };

  const handleDelete = async (id: number, name: string) => {
    if (!confirm(`Delete "${name}" from your landscape plan?`)) return;
    setDeletingId(id);
    try {
      await deletePlant(id);
      setPlants((prev) => prev.filter((p) => p.id !== id));
      setExpandedIds((prev) => {
        if (!prev.has(id)) return prev;
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete');
    } finally {
      setDeletingId(null);
    }
  };

  const handleRefresh = async (id: number) => {
    setRefreshingId(id);
    try {
      const updated = await refreshPlantPlan(id);
      setPlants((prev) => prev.map((p) => (p.id === id ? updated : p)));
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to regenerate plan');
    } finally {
      setRefreshingId(null);
    }
  };

  return (
    <div>
      <div style={sharedStyles.headerRow}>
        <h1 style={sharedStyles.pageTitle}>Landscape</h1>
        <button style={styles.addBtn} onClick={() => setShowForm(!showForm)}>
          {showForm ? 'Cancel' : '+ Add Plant'}
        </button>
      </div>

      <p style={styles.intro}>
        Track shrubs, bushes, and other plants alongside your turf. We'll generate a
        homeowner-level maintenance plan (pruning windows, fertilizing timing, etc.) and
        surface those windows in your Calendar, Seasonal Plan, and Recommendations.
      </p>

      {showForm && <AddPlantForm onCreated={handleCreated} onError={setError} />}

      {error && <div style={sharedStyles.error}>{error}</div>}

      {loading ? (
        <p style={sharedStyles.loading}>Loading plants...</p>
      ) : plants.length === 0 ? (
        <p style={sharedStyles.empty}>
          No plants yet. Click "+ Add Plant" to get started.
        </p>
      ) : (
        <div style={styles.list}>
          {sortedPlants.map((plant) => (
            <PlantRow
              key={plant.id ?? plant.common_name}
              plant={plant}
              expanded={plant.id != null && expandedIds.has(plant.id)}
              onToggle={() => plant.id != null && toggleExpanded(plant.id)}
              onRefresh={() => plant.id != null && handleRefresh(plant.id)}
              onDelete={() =>
                plant.id != null && handleDelete(plant.id, plant.common_name)
              }
              refreshing={refreshingId === plant.id}
              deleting={deletingId === plant.id}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PlantRow({
  plant,
  expanded,
  onToggle,
  onRefresh,
  onDelete,
  refreshing,
  deleting,
}: {
  plant: Plant;
  expanded: boolean;
  onToggle: () => void;
  onRefresh: () => void;
  onDelete: () => void;
  refreshing: boolean;
  deleting: boolean;
}) {
  const plan = plant.maintenance_plan;
  const confidenceColor =
    IDENTIFICATION_CONFIDENCE_COLORS[plan.identification_confidence];

  return (
    <div style={styles.row}>
      <button
        type="button"
        onClick={onToggle}
        style={styles.rowHeader}
        aria-expanded={expanded}
      >
        <span style={styles.chevron}>{expanded ? '▾' : '▸'}</span>
        <div style={styles.rowHeaderMain}>
          <div style={styles.rowTitleLine}>
            <span style={styles.plantName}>{plant.common_name}</span>
            {plant.scientific_name && (
              <span style={styles.sciName}>{plant.scientific_name}</span>
            )}
          </div>
          <div style={styles.metaRow}>
            <span style={styles.metaItem}>{PLANT_TYPE_LABELS[plant.plant_type]}</span>
            {plant.location && (
              <span style={styles.metaItem}>📍 {plant.location}</span>
            )}
            <span style={styles.metaItem}>{plan.tasks.length} tasks</span>
          </div>
          {!expanded && <div style={styles.rowSummary}>{plan.summary}</div>}
        </div>
        <span
          style={{
            ...sharedStyles.badge,
            backgroundColor: confidenceColor + '22',
            color: confidenceColor,
            borderColor: confidenceColor,
            flexShrink: 0,
          }}
          title="LLM identification confidence"
        >
          {plan.identification_confidence} confidence
        </span>
      </button>

      {expanded && (
        <div style={styles.rowBody}>
          <p style={styles.summary}>{plan.summary}</p>

          {plant.planting_date && (
            <div style={styles.bodyMeta}>Planted {plant.planting_date}</div>
          )}

          {plan.warnings.length > 0 && (
            <div style={styles.warningsBox}>
              {plan.warnings.map((w, i) => (
                <div key={i} style={styles.warning}>
                  ⚠ {w}
                </div>
              ))}
            </div>
          )}

          <h3 style={styles.tasksHeading}>Maintenance tasks ({plan.tasks.length})</h3>
          {plan.tasks.length === 0 ? (
            <p style={sharedStyles.empty}>No tasks in this plan.</p>
          ) : (
            <ul style={styles.taskList}>
              {plan.tasks.map((task, idx) => (
                <TaskRow key={idx} task={task} />
              ))}
            </ul>
          )}

          <div style={styles.rowFooter}>
            <div style={styles.footerMeta}>
              Plan generated {formatDate(plant.plan_generated_at)} · {plant.plan_model}
            </div>
            <div style={styles.footerButtons}>
              <button
                style={styles.secondaryBtn}
                onClick={onRefresh}
                disabled={refreshing}
              >
                {refreshing ? 'Regenerating…' : 'Regenerate plan'}
              </button>
              <button
                style={styles.deleteBtn}
                onClick={onDelete}
                disabled={deleting}
              >
                {deleting ? 'Deleting…' : 'Delete'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function TaskRow({ task }: { task: MaintenanceTask }) {
  return (
    <li style={styles.taskRow}>
      <div style={styles.taskHeader}>
        <span style={styles.taskType}>{TASK_TYPE_LABELS[task.task_type]}</span>
        <span style={styles.taskWindow}>
          {formatMmDd(task.window_start_month_day)} –{' '}
          {formatMmDd(task.window_end_month_day)}
        </span>
        <span style={styles.taskFrequency}>{task.frequency}</span>
      </div>
      <div style={styles.taskDescription}>{task.description}</div>
      {task.zone_note && <div style={styles.taskZoneNote}>{task.zone_note}</div>}
    </li>
  );
}

function AddPlantForm({
  onCreated,
  onError,
}: {
  onCreated: (plant: Plant) => void;
  onError: (msg: string) => void;
}) {
  const [input, setInput] = useState('');
  const [plantType, setPlantType] = useState<PlantType>('Shrub');
  const [location, setLocation] = useState('');
  const [plantingDate, setPlantingDate] = useState('');
  const [notes, setNotes] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    try {
      const trimmed = input.trim();
      // Heuristic: two-word "Genus species" with capital first letter → treat as scientific.
      const looksScientific =
        /^[A-Z][a-z]+\s+[a-z]+/.test(trimmed) && trimmed.split(/\s+/).length >= 2;
      const created = await createPlant({
        common_name: looksScientific ? undefined : trimmed,
        scientific_name: looksScientific ? trimmed : undefined,
        plant_type: plantType,
        location: location || undefined,
        planting_date: plantingDate || undefined,
        notes: notes || undefined,
      });
      onCreated(created);
    } catch (err) {
      onError(err instanceof Error ? err.message : 'Failed to create plant');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} style={styles.form}>
      <div style={styles.formGrid}>
        <div style={{ gridColumn: '1 / -1' }}>
          <label style={styles.formLabel}>
            Common name or scientific name (Genus species)
          </label>
          <input
            style={styles.input}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="e.g. Hydrangea paniculata, or 'panicle hydrangea'"
            required
          />
        </div>
        <div>
          <label style={styles.formLabel}>Plant type</label>
          <select
            style={styles.input}
            value={plantType}
            onChange={(e) => setPlantType(e.target.value as PlantType)}
          >
            {PLANT_TYPES.map((t) => (
              <option key={t} value={t}>
                {PLANT_TYPE_LABELS[t]}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label style={styles.formLabel}>Location (optional)</label>
          <input
            style={styles.input}
            value={location}
            onChange={(e) => setLocation(e.target.value)}
            placeholder="e.g. front bed"
          />
        </div>
        <div>
          <label style={styles.formLabel}>Planting date (optional)</label>
          <input
            type="date"
            style={styles.input}
            value={plantingDate}
            onChange={(e) => setPlantingDate(e.target.value)}
          />
        </div>
        <div style={{ gridColumn: '1 / -1' }}>
          <label style={styles.formLabel}>Notes (optional)</label>
          <input
            style={styles.input}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
          />
        </div>
      </div>
      <button type="submit" style={styles.submitBtn} disabled={submitting}>
        {submitting ? 'Generating plan…' : 'Add plant & generate plan'}
      </button>
      <div style={styles.submitHint}>
        Plan generation calls the configured LLM and may take 10–30 seconds.
      </div>
    </form>
  );
}

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString();
  } catch {
    return iso;
  }
}

function formatMmDd(mmdd: string): string {
  const [mStr, dStr] = mmdd.split('-');
  const month = parseInt(mStr, 10);
  const day = parseInt(dStr, 10);
  if (Number.isNaN(month) || Number.isNaN(day)) return mmdd;
  const d = new Date(2000, month - 1, day);
  return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

const styles: Record<string, React.CSSProperties> = {
  intro: { color: '#4a5568', fontSize: '0.9rem', marginBottom: '1rem' },
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
  list: {
    display: 'flex',
    flexDirection: 'column',
    gap: '0.5rem',
    maxWidth: 960,
  },
  row: {
    backgroundColor: '#fff',
    borderRadius: 8,
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
    overflow: 'hidden',
  },
  rowHeader: {
    display: 'flex',
    alignItems: 'flex-start',
    gap: '0.75rem',
    width: '100%',
    padding: '0.75rem 1rem',
    backgroundColor: 'transparent',
    border: 'none',
    cursor: 'pointer',
    textAlign: 'left',
    fontFamily: 'inherit',
  },
  chevron: {
    fontSize: '0.85rem',
    color: '#4a5568',
    marginTop: 4,
    width: 12,
    flexShrink: 0,
  },
  rowHeaderMain: {
    flex: 1,
    minWidth: 0,
  },
  rowTitleLine: {
    display: 'flex',
    alignItems: 'baseline',
    gap: '0.5rem',
    flexWrap: 'wrap',
  },
  rowSummary: {
    fontSize: '0.8rem',
    color: '#4a5568',
    marginTop: 4,
  },
  rowBody: {
    padding: '0.75rem 1rem 1rem',
    borderTop: '1px solid #edf2f7',
  },
  bodyMeta: {
    fontSize: '0.75rem',
    color: '#4a5568',
    marginBottom: '0.5rem',
  },
  plantName: { fontSize: '1.05rem', color: '#1a202c', fontWeight: 600 },
  sciName: { fontSize: '0.8rem', color: '#718096', fontStyle: 'italic' },
  metaRow: {
    display: 'flex',
    flexWrap: 'wrap',
    gap: '0.5rem',
    marginTop: 2,
  },
  metaItem: { fontSize: '0.75rem', color: '#4a5568' },
  summary: { fontSize: '0.85rem', color: '#2d3748', marginBottom: '0.75rem', marginTop: 0 },
  warningsBox: {
    backgroundColor: '#fefce8',
    border: '1px solid #fde68a',
    borderRadius: 6,
    padding: '0.5rem 0.75rem',
    marginBottom: '0.75rem',
  },
  warning: { fontSize: '0.8rem', color: '#713f12' },
  tasksHeading: {
    fontSize: '0.85rem',
    color: '#2d3748',
    fontWeight: 600,
    margin: '0 0 0.5rem',
  },
  taskList: { listStyle: 'none', margin: 0, padding: 0, marginBottom: '0.75rem' },
  taskRow: {
    padding: '0.5rem 0',
    borderBottom: '1px solid #edf2f7',
  },
  taskHeader: {
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
    marginBottom: 2,
    flexWrap: 'wrap',
  },
  taskType: { fontWeight: 600, fontSize: '0.8rem', color: '#1a202c' },
  taskWindow: { fontSize: '0.75rem', color: '#4a5568' },
  taskFrequency: {
    fontSize: '0.7rem',
    color: '#718096',
    backgroundColor: '#edf2f7',
    padding: '1px 6px',
    borderRadius: 8,
  },
  taskDescription: { fontSize: '0.8rem', color: '#2d3748' },
  taskZoneNote: {
    fontSize: '0.75rem',
    color: '#718096',
    fontStyle: 'italic',
    marginTop: 2,
  },
  rowFooter: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: '0.5rem',
    paddingTop: '0.5rem',
    borderTop: '1px solid #edf2f7',
    flexWrap: 'wrap',
  },
  footerMeta: { fontSize: '0.7rem', color: '#a0aec0' },
  footerButtons: { display: 'flex', gap: '0.5rem' },
  secondaryBtn: {
    padding: '4px 10px',
    backgroundColor: 'transparent',
    color: '#3182ce',
    border: '1px solid #3182ce',
    borderRadius: 4,
    cursor: 'pointer',
    fontSize: '0.75rem',
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
  submitHint: {
    fontSize: '0.75rem',
    color: '#a0aec0',
    marginTop: 6,
  },
};
