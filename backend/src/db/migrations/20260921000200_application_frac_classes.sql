-- FRAC classes of a logged fungicide, as chosen on the form (e.g. {Frac11,Frac3} for an
-- azoxystrobin + propiconazole premix). NULL = not recorded: fall back to recognising the
-- product name, which is all there was before and silently ignored anything it didn't know.
ALTER TABLE applications ADD COLUMN IF NOT EXISTS frac_classes TEXT[];
