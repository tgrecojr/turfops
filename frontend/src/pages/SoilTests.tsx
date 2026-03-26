import { useCallback, useEffect, useState } from 'react';
import {
  createSoilTest,
  updateSoilTest,
  deleteSoilTest,
  getSoilTests,
  getSoilTestRecommendations,
} from '../api/client';
import type {
  SoilTest,
  SoilTestSummary,
  NutrientLevel,
} from '../types';
import { NUTRIENT_LEVEL_COLORS } from '../types';
import { sharedStyles } from '../styles/shared';

export default function SoilTests() {
  const [tests, setTests] = useState<SoilTest[]>([]);
  const [summary, setSummary] = useState<SoilTestSummary | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [formOpen, setFormOpen] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [submitting, setSubmitting] = useState(false);

  // Form fields
  const [testDate, setTestDate] = useState(new Date().toISOString().slice(0, 10));
  const [labName, setLabName] = useState('');
  const [ph, setPh] = useState('');
  const [bufferPh, setBufferPh] = useState('');
  const [phosphorus, setPhosphorus] = useState('');
  const [potassium, setPotassium] = useState('');
  const [calcium, setCalcium] = useState('');
  const [magnesium, setMagnesium] = useState('');
  const [sulfur, setSulfur] = useState('');
  const [iron, setIron] = useState('');
  const [manganese, setManganese] = useState('');
  const [zinc, setZinc] = useState('');
  const [boron, setBoron] = useState('');
  const [copper, setCopper] = useState('');
  const [organicMatter, setOrganicMatter] = useState('');
  const [cec, setCec] = useState('');
  const [notes, setNotes] = useState('');

  const loadData = useCallback(async () => {
    try {
      setError('');
      const t = await getSoilTests();
      setTests(t);
      if (t.length > 0) {
        const s = await getSoilTestRecommendations();
        setSummary(s);
      } else {
        setSummary(null);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load data');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const resetForm = () => {
    setEditingId(null);
    setTestDate(new Date().toISOString().slice(0, 10));
    setLabName('');
    setPh('');
    setBufferPh('');
    setPhosphorus('');
    setPotassium('');
    setCalcium('');
    setMagnesium('');
    setSulfur('');
    setIron('');
    setManganese('');
    setZinc('');
    setBoron('');
    setCopper('');
    setOrganicMatter('');
    setCec('');
    setNotes('');
  };

  const handleEdit = (t: SoilTest) => {
    setEditingId(t.id ?? null);
    setTestDate(t.test_date);
    setLabName(t.lab_name ?? '');
    setPh(String(t.ph));
    setBufferPh(t.buffer_ph != null ? String(t.buffer_ph) : '');
    setPhosphorus(t.phosphorus_ppm != null ? String(t.phosphorus_ppm) : '');
    setPotassium(t.potassium_ppm != null ? String(t.potassium_ppm) : '');
    setCalcium(t.calcium_ppm != null ? String(t.calcium_ppm) : '');
    setMagnesium(t.magnesium_ppm != null ? String(t.magnesium_ppm) : '');
    setSulfur(t.sulfur_ppm != null ? String(t.sulfur_ppm) : '');
    setIron(t.iron_ppm != null ? String(t.iron_ppm) : '');
    setManganese(t.manganese_ppm != null ? String(t.manganese_ppm) : '');
    setZinc(t.zinc_ppm != null ? String(t.zinc_ppm) : '');
    setBoron(t.boron_ppm != null ? String(t.boron_ppm) : '');
    setCopper(t.copper_ppm != null ? String(t.copper_ppm) : '');
    setOrganicMatter(t.organic_matter_pct != null ? String(t.organic_matter_pct) : '');
    setCec(t.cec != null ? String(t.cec) : '');
    setNotes(t.notes ?? '');
    setFormOpen(true);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!ph) { setError('pH is required'); return; }
    setSubmitting(true);
    setError('');
    try {
      const optNum = (v: string) => v ? Number(v) : undefined;
      const data = {
        test_date: testDate,
        lab_name: labName || undefined,
        ph: Number(ph),
        buffer_ph: optNum(bufferPh),
        phosphorus_ppm: optNum(phosphorus),
        potassium_ppm: optNum(potassium),
        calcium_ppm: optNum(calcium),
        magnesium_ppm: optNum(magnesium),
        sulfur_ppm: optNum(sulfur),
        iron_ppm: optNum(iron),
        manganese_ppm: optNum(manganese),
        zinc_ppm: optNum(zinc),
        boron_ppm: optNum(boron),
        copper_ppm: optNum(copper),
        organic_matter_pct: optNum(organicMatter),
        cec: optNum(cec),
        notes: notes || undefined,
      };
      if (editingId) {
        await updateSoilTest(editingId, data);
      } else {
        await createSoilTest(data);
      }
      resetForm();
      setFormOpen(false);
      await loadData();
    } catch (e) {
      setError(e instanceof Error ? e.message : editingId ? 'Failed to update soil test' : 'Failed to create soil test');
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteSoilTest(id);
      await loadData();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete');
    }
  };

  if (loading) return <div style={sharedStyles.loading}>Loading soil tests...</div>;

  return (
    <div>
      <div style={sharedStyles.headerRow}>
        <h1 style={sharedStyles.pageTitle}>Soil Tests</h1>
        <button onClick={() => { if (formOpen) { resetForm(); setFormOpen(false); } else { setFormOpen(true); } }} style={styles.addBtn}>
          {formOpen ? 'Cancel' : '+ Add Soil Test'}
        </button>
      </div>

      {error && <div style={sharedStyles.error}>{error}</div>}

      {formOpen && (
        <form onSubmit={handleSubmit} style={{ ...sharedStyles.card, marginBottom: '1.5rem' }}>
          <h3 style={sharedStyles.sectionTitle}>{editingId ? 'Edit Soil Test' : 'New Soil Test'}</h3>

          <div style={styles.formSection}>
            <h4 style={styles.formSectionTitle}>Core</h4>
            <div style={styles.formGrid}>
              <label style={styles.label}>
                Date *
                <input type="date" value={testDate} onChange={e => setTestDate(e.target.value)} required style={styles.input} />
              </label>
              <label style={styles.label}>
                Lab Name
                <input type="text" value={labName} onChange={e => setLabName(e.target.value)} placeholder="e.g. Waypoint" style={styles.input} />
              </label>
              <label style={styles.label}>
                pH *
                <input type="number" step="0.1" min="0" max="14" value={ph} onChange={e => setPh(e.target.value)} required style={styles.input} />
              </label>
              <label style={styles.label}>
                Buffer pH
                <input type="number" step="0.1" value={bufferPh} onChange={e => setBufferPh(e.target.value)} style={styles.input} />
              </label>
            </div>
          </div>

          <div style={styles.formSection}>
            <h4 style={styles.formSectionTitle}>Macronutrients (ppm)</h4>
            <div style={styles.formGrid}>
              <label style={styles.label}>
                Phosphorus (P)
                <input type="number" step="0.1" value={phosphorus} onChange={e => setPhosphorus(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Potassium (K)
                <input type="number" step="0.1" value={potassium} onChange={e => setPotassium(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Calcium (Ca)
                <input type="number" step="0.1" value={calcium} onChange={e => setCalcium(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Magnesium (Mg)
                <input type="number" step="0.1" value={magnesium} onChange={e => setMagnesium(e.target.value)} style={styles.input} />
              </label>
            </div>
          </div>

          <div style={styles.formSection}>
            <h4 style={styles.formSectionTitle}>Micronutrients (ppm)</h4>
            <div style={styles.formGrid}>
              <label style={styles.label}>
                Sulfur (S)
                <input type="number" step="0.1" value={sulfur} onChange={e => setSulfur(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Iron (Fe)
                <input type="number" step="0.1" value={iron} onChange={e => setIron(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Manganese (Mn)
                <input type="number" step="0.1" value={manganese} onChange={e => setManganese(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Zinc (Zn)
                <input type="number" step="0.01" value={zinc} onChange={e => setZinc(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Boron (B)
                <input type="number" step="0.01" value={boron} onChange={e => setBoron(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                Copper (Cu)
                <input type="number" step="0.01" value={copper} onChange={e => setCopper(e.target.value)} style={styles.input} />
              </label>
            </div>
          </div>

          <div style={styles.formSection}>
            <h4 style={styles.formSectionTitle}>Other</h4>
            <div style={styles.formGrid}>
              <label style={styles.label}>
                Organic Matter %
                <input type="number" step="0.1" value={organicMatter} onChange={e => setOrganicMatter(e.target.value)} style={styles.input} />
              </label>
              <label style={styles.label}>
                CEC (meq/100g)
                <input type="number" step="0.1" value={cec} onChange={e => setCec(e.target.value)} style={styles.input} />
              </label>
              <label style={{ ...styles.label, gridColumn: '1 / -1' }}>
                Notes
                <input type="text" value={notes} onChange={e => setNotes(e.target.value)} style={styles.input} />
              </label>
            </div>
          </div>

          <button type="submit" disabled={submitting} style={styles.submitBtn}>
            {submitting ? 'Saving...' : editingId ? 'Update Soil Test' : 'Save Soil Test'}
          </button>
        </form>
      )}

      {/* Recommendations Panel */}
      {summary && <SoilTestRecommendationsPanel summary={summary} />}

      {/* Historical Tests Table */}
      <h2 style={sharedStyles.sectionTitle}>Test History</h2>
      {tests.length === 0 ? (
        <div style={sharedStyles.empty}>No soil tests recorded yet. Add your first test above.</div>
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table style={sharedStyles.table}>
            <thead>
              <tr>
                <th style={sharedStyles.th}>Date</th>
                <th style={sharedStyles.th}>Lab</th>
                <th style={sharedStyles.th}>pH</th>
                <th style={sharedStyles.th}>P (ppm)</th>
                <th style={sharedStyles.th}>K (ppm)</th>
                <th style={sharedStyles.th}>Ca (ppm)</th>
                <th style={sharedStyles.th}>Mg (ppm)</th>
                <th style={sharedStyles.th}>OM %</th>
                <th style={sharedStyles.th}></th>
              </tr>
            </thead>
            <tbody>
              {tests.map(t => (
                <tr key={t.id}>
                  <td style={sharedStyles.td}>{t.test_date}</td>
                  <td style={sharedStyles.td}>{t.lab_name ?? '-'}</td>
                  <td style={sharedStyles.td}>{t.ph.toFixed(1)}</td>
                  <td style={sharedStyles.td}>{t.phosphorus_ppm?.toFixed(0) ?? '-'}</td>
                  <td style={sharedStyles.td}>{t.potassium_ppm?.toFixed(0) ?? '-'}</td>
                  <td style={sharedStyles.td}>{t.calcium_ppm?.toFixed(0) ?? '-'}</td>
                  <td style={sharedStyles.td}>{t.magnesium_ppm?.toFixed(0) ?? '-'}</td>
                  <td style={sharedStyles.td}>{t.organic_matter_pct?.toFixed(1) ?? '-'}</td>
                  <td style={{ ...sharedStyles.td, display: 'flex', gap: '0.5rem' }}>
                    <button onClick={() => handleEdit(t)} style={styles.editBtn}>Edit</button>
                    <button onClick={() => t.id && handleDelete(t.id)} style={styles.deleteBtn}>Delete</button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function LevelBadge({ level }: { level: NutrientLevel }) {
  const color = NUTRIENT_LEVEL_COLORS[level];
  return (
    <span style={{
      ...sharedStyles.badge,
      backgroundColor: color + '22',
      color,
      borderColor: color,
    }}>
      {level}
    </span>
  );
}

function SoilTestRecommendationsPanel({ summary }: { summary: SoilTestSummary }) {
  const { ph_recommendation, npk_recommendation, micronutrient_recommendations } = summary;
  const hasRecs = ph_recommendation || npk_recommendation || micronutrient_recommendations.length > 0;

  if (!hasRecs) {
    return (
      <div style={{ ...sharedStyles.card, marginBottom: '1.5rem' }}>
        <h2 style={sharedStyles.sectionTitle}>Recommendations</h2>
        <p style={{ color: '#718096', fontSize: '0.9rem' }}>
          All soil nutrient levels are adequate. No amendments recommended.
        </p>
      </div>
    );
  }

  return (
    <div style={{ marginBottom: '1.5rem' }}>
      <h2 style={sharedStyles.sectionTitle}>Recommendations</h2>
      <div style={{ display: 'grid', gap: '1rem', gridTemplateColumns: 'repeat(auto-fit, minmax(320px, 1fr))' }}>

        {ph_recommendation && (
          <div style={sharedStyles.card}>
            <h3 style={styles.recTitle}>pH Adjustment</h3>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>Current pH</span>
              <span style={styles.recValue}>{ph_recommendation.current_ph.toFixed(1)}</span>
            </div>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>Target pH</span>
              <span style={styles.recValue}>{ph_recommendation.target_ph.toFixed(1)}</span>
            </div>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>Amendment</span>
              <span style={styles.recValue}>{ph_recommendation.amendment}</span>
            </div>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>Rate</span>
              <span style={{ ...styles.recValue, fontWeight: 700 }}>
                {ph_recommendation.rate_lbs_per_1000sqft.toFixed(0)} lbs/1000 sqft
              </span>
            </div>
            <p style={styles.recExplanation}>{ph_recommendation.explanation}</p>
          </div>
        )}

        {npk_recommendation && (
          <div style={sharedStyles.card}>
            <h3 style={styles.recTitle}>Fertilizer Recommendation</h3>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>Recommended Ratio</span>
              <span style={{ ...styles.recValue, fontWeight: 700, fontSize: '1.1rem' }}>
                {npk_recommendation.recommended_ratio}
              </span>
            </div>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>N rate</span>
              <span style={styles.recValue}>{npk_recommendation.nitrogen_rate_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft</span>
            </div>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>P₂O₅ rate</span>
              <span style={styles.recValue}>
                {npk_recommendation.phosphorus_rate_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft
                <LevelBadge level={npk_recommendation.phosphorus_level} />
              </span>
            </div>
            <div style={styles.recRow}>
              <span style={styles.recLabel}>K₂O rate</span>
              <span style={styles.recValue}>
                {npk_recommendation.potassium_rate_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft
                <LevelBadge level={npk_recommendation.potassium_level} />
              </span>
            </div>
            {npk_recommendation.example_product_ratio !== 'N/A' && (
              <div style={styles.recRow}>
                <span style={styles.recLabel}>Example Product</span>
                <span style={styles.recValue}>
                  {npk_recommendation.example_product_ratio} at {npk_recommendation.product_rate_lbs_per_1000sqft.toFixed(1)} lbs/1000sqft
                </span>
              </div>
            )}
            <div style={{ ...styles.recRow, marginTop: '0.5rem', padding: '0.5rem', backgroundColor: '#f7fafc', borderRadius: 4 }}>
              <span style={styles.recLabel}>N Budget Remaining</span>
              <span style={{ ...styles.recValue, fontWeight: 700 }}>
                {npk_recommendation.remaining_n_budget_lbs_per_1000sqft.toFixed(2)} lbs/1000sqft
              </span>
            </div>
            <p style={styles.recExplanation}>{npk_recommendation.explanation}</p>
          </div>
        )}
      </div>

      {micronutrient_recommendations.length > 0 && (
        <div style={{ ...sharedStyles.card, marginTop: '1rem' }}>
          <h3 style={styles.recTitle}>Micronutrient Deficiencies</h3>
          <table style={sharedStyles.table}>
            <thead>
              <tr>
                <th style={sharedStyles.th}>Nutrient</th>
                <th style={sharedStyles.th}>Current (ppm)</th>
                <th style={sharedStyles.th}>Threshold (ppm)</th>
                <th style={sharedStyles.th}>Status</th>
                <th style={sharedStyles.th}>Suggestion</th>
              </tr>
            </thead>
            <tbody>
              {micronutrient_recommendations.map(m => (
                <tr key={m.nutrient}>
                  <td style={sharedStyles.td}>{m.nutrient}</td>
                  <td style={sharedStyles.td}>{m.current_ppm.toFixed(1)}</td>
                  <td style={sharedStyles.td}>{m.threshold_ppm.toFixed(1)}</td>
                  <td style={sharedStyles.td}><LevelBadge level={m.level} /></td>
                  <td style={{ ...sharedStyles.td, fontSize: '0.8rem' }}>{m.suggestion}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  addBtn: {
    padding: '0.5rem 1rem',
    backgroundColor: '#38a169',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    cursor: 'pointer',
    fontWeight: 600,
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
    marginTop: '0.5rem',
  },
  editBtn: {
    padding: '2px 8px',
    backgroundColor: 'transparent',
    color: '#3182ce',
    border: '1px solid #3182ce',
    borderRadius: 4,
    cursor: 'pointer',
    fontSize: '0.75rem',
  },
  deleteBtn: {
    padding: '2px 8px',
    backgroundColor: 'transparent',
    color: '#e53e3e',
    border: '1px solid #e53e3e',
    borderRadius: 4,
    cursor: 'pointer',
    fontSize: '0.75rem',
  },
  formSection: {
    marginBottom: '1rem',
  },
  formSectionTitle: {
    fontSize: '0.85rem',
    fontWeight: 600,
    color: '#4a5568',
    margin: '0 0 0.5rem',
    borderBottom: '1px solid #e2e8f0',
    paddingBottom: '0.25rem',
  },
  formGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fill, minmax(180px, 1fr))',
    gap: '0.75rem',
  },
  label: {
    display: 'flex',
    flexDirection: 'column' as const,
    fontSize: '0.8rem',
    color: '#4a5568',
    fontWeight: 500,
    gap: '0.25rem',
  },
  input: {
    padding: '0.4rem 0.5rem',
    border: '1px solid #e2e8f0',
    borderRadius: 4,
    fontSize: '0.85rem',
  },
  recTitle: {
    fontSize: '0.95rem',
    fontWeight: 700,
    color: '#2d3748',
    margin: '0 0 0.75rem',
  },
  recRow: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '0.25rem 0',
    gap: '0.5rem',
  },
  recLabel: {
    fontSize: '0.8rem',
    color: '#718096',
  },
  recValue: {
    fontSize: '0.85rem',
    color: '#2d3748',
    display: 'flex',
    alignItems: 'center',
    gap: '0.5rem',
  },
  recExplanation: {
    fontSize: '0.8rem',
    color: '#718096',
    marginTop: '0.75rem',
    lineHeight: 1.5,
    borderTop: '1px solid #e2e8f0',
    paddingTop: '0.5rem',
  },
};
