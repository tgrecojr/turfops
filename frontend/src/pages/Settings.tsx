import { useEffect, useState } from 'react';
import { getProfile, updateProfile } from '../api/client';
import type { GrassType, IrrigationType, LawnProfile, SoilType } from '../types';
import { GRASS_TYPE_LABELS } from '../types';

const GRASS_TYPES: GrassType[] = [
  'KentuckyBluegrass',
  'TallFescue',
  'PerennialRyegrass',
  'FineFescue',
  'Bermuda',
  'Zoysia',
  'StAugustine',
  'Mixed',
];

const SOIL_TYPES: SoilType[] = [
  'Clay',
  'Loam',
  'Sandy',
  'SiltLoam',
  'ClayLoam',
  'SandyLoam',
];

const IRRIGATION_TYPES: IrrigationType[] = ['InGround', 'Hose', 'None'];

const IRRIGATION_LABELS: Record<IrrigationType, string> = {
  InGround: 'In-Ground',
  Hose: 'Hose/Sprinkler',
  None: 'None',
};

const SOIL_LABELS: Record<SoilType, string> = {
  Clay: 'Clay',
  Loam: 'Loam',
  Sandy: 'Sandy',
  SiltLoam: 'Silt Loam',
  ClayLoam: 'Clay Loam',
  SandyLoam: 'Sandy Loam',
};

export default function Settings() {
  const [profile, setProfile] = useState<LawnProfile | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  // Form state
  const [name, setName] = useState('');
  const [grassType, setGrassType] = useState<string>('TallFescue');
  const [zone, setZone] = useState('');
  const [soilType, setSoilType] = useState<string>('');
  const [size, setSize] = useState('');
  const [irrigationType, setIrrigationType] = useState<string>('');

  useEffect(() => {
    (async () => {
      try {
        const p = await getProfile();
        setProfile(p);
        setName(p.name);
        setGrassType(p.grass_type);
        setZone(p.usda_zone);
        setSoilType(p.soil_type || '');
        setSize(p.lawn_size_sqft?.toString() || '');
        setIrrigationType(p.irrigation_type || '');
      } catch (e) {
        setError(e instanceof Error ? e.message : 'Failed to load profile');
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError(null);
    setSuccess(false);

    try {
      const validGrassType = GRASS_TYPES.includes(grassType as GrassType)
        ? (grassType as GrassType)
        : undefined;
      const validSoilType =
        soilType && SOIL_TYPES.includes(soilType as SoilType)
          ? (soilType as SoilType)
          : undefined;
      const validIrrigationType =
        irrigationType && IRRIGATION_TYPES.includes(irrigationType as IrrigationType)
          ? (irrigationType as IrrigationType)
          : undefined;

      if (!validGrassType) {
        setError('Please select a valid grass type');
        setSaving(false);
        return;
      }

      const updated = await updateProfile({
        name,
        grass_type: validGrassType,
        usda_zone: zone,
        soil_type: validSoilType,
        lawn_size_sqft: size ? parseFloat(size) : undefined,
        irrigation_type: validIrrigationType,
      });
      setProfile(updated);
      setSuccess(true);
      setTimeout(() => setSuccess(false), 3000);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to save');
    } finally {
      setSaving(false);
    }
  };

  if (loading)
    return <div style={{ color: '#718096', padding: '2rem' }}>Loading...</div>;

  return (
    <div>
      <h1 style={styles.title}>Lawn Profile Settings</h1>

      {error && <div style={styles.error}>{error}</div>}
      {success && <div style={styles.success}>Profile saved successfully!</div>}

      <form onSubmit={handleSave} style={styles.form}>
        <div style={styles.grid}>
          <div>
            <label style={styles.label}>Lawn Name</label>
            <input
              style={styles.input}
              value={name}
              onChange={(e) => setName(e.target.value)}
              required
            />
          </div>
          <div>
            <label style={styles.label}>Grass Type</label>
            <select
              style={styles.input}
              value={grassType}
              onChange={(e) => setGrassType(e.target.value)}
            >
              {GRASS_TYPES.map((gt) => (
                <option key={gt} value={gt}>
                  {GRASS_TYPE_LABELS[gt]}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label style={styles.label}>USDA Zone</label>
            <input
              style={styles.input}
              value={zone}
              onChange={(e) => setZone(e.target.value)}
              placeholder="e.g. 7a"
              required
            />
          </div>
          <div>
            <label style={styles.label}>Soil Type</label>
            <select
              style={styles.input}
              value={soilType}
              onChange={(e) => setSoilType(e.target.value)}
            >
              <option value="">Not specified</option>
              {SOIL_TYPES.map((st) => (
                <option key={st} value={st}>
                  {SOIL_LABELS[st]}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label style={styles.label}>Lawn Size (sqft)</label>
            <input
              type="number"
              style={styles.input}
              value={size}
              onChange={(e) => setSize(e.target.value)}
              placeholder="e.g. 5000"
            />
          </div>
          <div>
            <label style={styles.label}>Irrigation Type</label>
            <select
              style={styles.input}
              value={irrigationType}
              onChange={(e) => setIrrigationType(e.target.value)}
            >
              <option value="">Not specified</option>
              {IRRIGATION_TYPES.map((it) => (
                <option key={it} value={it}>
                  {IRRIGATION_LABELS[it]}
                </option>
              ))}
            </select>
          </div>
        </div>
        <button type="submit" style={styles.saveBtn} disabled={saving}>
          {saving ? 'Saving...' : 'Save Profile'}
        </button>
      </form>

      {profile && (
        <div style={styles.meta}>
          Created: {new Date(profile.created_at).toLocaleDateString()} | Last
          updated: {new Date(profile.updated_at).toLocaleString()}
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
  success: {
    padding: '0.5rem 1rem',
    backgroundColor: '#c6f6d5',
    color: '#276749',
    borderRadius: 6,
    marginBottom: '1rem',
    fontSize: '0.85rem',
  },
  form: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: '1.5rem',
    boxShadow: '0 1px 3px rgba(0,0,0,0.08)',
    marginBottom: '1rem',
  },
  grid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))',
    gap: '1rem',
    marginBottom: '1rem',
  },
  label: {
    display: 'block',
    fontSize: '0.8rem',
    color: '#718096',
    marginBottom: 4,
    fontWeight: 600,
  },
  input: {
    width: '100%',
    padding: '0.5rem 0.75rem',
    borderRadius: 6,
    border: '1px solid #e2e8f0',
    fontSize: '0.9rem',
  },
  saveBtn: {
    padding: '0.6rem 2rem',
    backgroundColor: '#3182ce',
    color: '#fff',
    border: 'none',
    borderRadius: 6,
    cursor: 'pointer',
    fontWeight: 600,
    fontSize: '0.9rem',
  },
  meta: {
    fontSize: '0.75rem',
    color: '#a0aec0',
  },
};
