use crate::error::Result;
use crate::models::SoilTest;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

pub async fn get_soil_tests_for_profile(
    pool: &PgPool,
    profile_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<SoilTest>> {
    let rows = sqlx::query_as::<_, SoilTestRow>(
        r#"SELECT id, lawn_profile_id, test_date, lab_name, ph, buffer_ph,
           phosphorus_ppm, potassium_ppm, calcium_ppm, magnesium_ppm,
           sulfur_ppm, iron_ppm, manganese_ppm, zinc_ppm, boron_ppm, copper_ppm,
           organic_matter_pct, cec, notes, created_at
           FROM soil_tests WHERE lawn_profile_id = $1
           ORDER BY test_date DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(profile_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_soil_test()).collect())
}

pub async fn get_latest_soil_test(pool: &PgPool, profile_id: i64) -> Result<Option<SoilTest>> {
    let row = sqlx::query_as::<_, SoilTestRow>(
        r#"SELECT id, lawn_profile_id, test_date, lab_name, ph, buffer_ph,
           phosphorus_ppm, potassium_ppm, calcium_ppm, magnesium_ppm,
           sulfur_ppm, iron_ppm, manganese_ppm, zinc_ppm, boron_ppm, copper_ppm,
           organic_matter_pct, cec, notes, created_at
           FROM soil_tests WHERE lawn_profile_id = $1
           ORDER BY test_date DESC LIMIT 1"#,
    )
    .bind(profile_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.into_soil_test()))
}

pub async fn create_soil_test(pool: &PgPool, test: &SoilTest) -> Result<i64> {
    let row = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO soil_tests
            (lawn_profile_id, test_date, lab_name, ph, buffer_ph,
             phosphorus_ppm, potassium_ppm, calcium_ppm, magnesium_ppm,
             sulfur_ppm, iron_ppm, manganese_ppm, zinc_ppm, boron_ppm, copper_ppm,
             organic_matter_pct, cec, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        RETURNING id
        "#,
    )
    .bind(test.lawn_profile_id)
    .bind(test.test_date)
    .bind(&test.lab_name)
    .bind(test.ph)
    .bind(test.buffer_ph)
    .bind(test.phosphorus_ppm)
    .bind(test.potassium_ppm)
    .bind(test.calcium_ppm)
    .bind(test.magnesium_ppm)
    .bind(test.sulfur_ppm)
    .bind(test.iron_ppm)
    .bind(test.manganese_ppm)
    .bind(test.zinc_ppm)
    .bind(test.boron_ppm)
    .bind(test.copper_ppm)
    .bind(test.organic_matter_pct)
    .bind(test.cec)
    .bind(&test.notes)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn update_soil_test(pool: &PgPool, id: i64, test: &SoilTest) -> Result<SoilTest> {
    let row = sqlx::query_as::<_, SoilTestRow>(
        r#"
        UPDATE soil_tests SET
            test_date = $2, lab_name = $3, ph = $4, buffer_ph = $5,
            phosphorus_ppm = $6, potassium_ppm = $7, calcium_ppm = $8, magnesium_ppm = $9,
            sulfur_ppm = $10, iron_ppm = $11, manganese_ppm = $12, zinc_ppm = $13,
            boron_ppm = $14, copper_ppm = $15, organic_matter_pct = $16, cec = $17, notes = $18
        WHERE id = $1
        RETURNING id, lawn_profile_id, test_date, lab_name, ph, buffer_ph,
            phosphorus_ppm, potassium_ppm, calcium_ppm, magnesium_ppm,
            sulfur_ppm, iron_ppm, manganese_ppm, zinc_ppm, boron_ppm, copper_ppm,
            organic_matter_pct, cec, notes, created_at
        "#,
    )
    .bind(id)
    .bind(test.test_date)
    .bind(&test.lab_name)
    .bind(test.ph)
    .bind(test.buffer_ph)
    .bind(test.phosphorus_ppm)
    .bind(test.potassium_ppm)
    .bind(test.calcium_ppm)
    .bind(test.magnesium_ppm)
    .bind(test.sulfur_ppm)
    .bind(test.iron_ppm)
    .bind(test.manganese_ppm)
    .bind(test.zinc_ppm)
    .bind(test.boron_ppm)
    .bind(test.copper_ppm)
    .bind(test.organic_matter_pct)
    .bind(test.cec)
    .bind(&test.notes)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(r.into_soil_test()),
        None => Err(crate::error::TurfOpsError::NotFound(format!(
            "Soil test {id} not found"
        ))),
    }
}

pub async fn delete_soil_test(pool: &PgPool, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM soil_tests WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct SoilTestRow {
    id: i64,
    lawn_profile_id: i64,
    test_date: NaiveDate,
    lab_name: Option<String>,
    ph: f64,
    buffer_ph: Option<f64>,
    phosphorus_ppm: Option<f64>,
    potassium_ppm: Option<f64>,
    calcium_ppm: Option<f64>,
    magnesium_ppm: Option<f64>,
    sulfur_ppm: Option<f64>,
    iron_ppm: Option<f64>,
    manganese_ppm: Option<f64>,
    zinc_ppm: Option<f64>,
    boron_ppm: Option<f64>,
    copper_ppm: Option<f64>,
    organic_matter_pct: Option<f64>,
    cec: Option<f64>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
}

impl SoilTestRow {
    fn into_soil_test(self) -> SoilTest {
        SoilTest {
            id: Some(self.id),
            lawn_profile_id: self.lawn_profile_id,
            test_date: self.test_date,
            lab_name: self.lab_name,
            ph: self.ph,
            buffer_ph: self.buffer_ph,
            phosphorus_ppm: self.phosphorus_ppm,
            potassium_ppm: self.potassium_ppm,
            calcium_ppm: self.calcium_ppm,
            magnesium_ppm: self.magnesium_ppm,
            sulfur_ppm: self.sulfur_ppm,
            iron_ppm: self.iron_ppm,
            manganese_ppm: self.manganese_ppm,
            zinc_ppm: self.zinc_ppm,
            boron_ppm: self.boron_ppm,
            copper_ppm: self.copper_ppm,
            organic_matter_pct: self.organic_matter_pct,
            cec: self.cec,
            notes: self.notes,
            created_at: self.created_at,
        }
    }
}
