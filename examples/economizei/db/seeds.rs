//! Database seeds — run with `doido db seed`.
//!
//! Default user:
//!   email:    admin@economizei.local
//!   password: password
//!
//! Runs in-process from the app binary, so its SQL is logged.

use doido::model::sea_orm::{ConnectionTrait, DatabaseConnection};

const SEED_SQL: &str = r#"
INSERT INTO users (email, password_digest, created_at, updated_at)
VALUES (
    'admin@economizei.local',
    '$2b$12$Siva8FwWJhKqR1kXV4RzvOMmoMxQMLPzE8IxmXZ90gAOxCDaPKMEi',
    NOW(),
    NOW()
)
ON CONFLICT (email) DO NOTHING;

INSERT INTO companies (name, slug)
SELECT 'Demo Company', 'demo'
WHERE NOT EXISTS (
    SELECT 1 FROM companies WHERE slug = 'demo'
);

INSERT INTO memberships (user_id, company_id, role, salary)
SELECT u.id, c.id, 'owner', NULL
FROM users u
INNER JOIN companies c ON c.slug = 'demo'
WHERE u.email = 'admin@economizei.local'
  AND NOT EXISTS (
      SELECT 1
      FROM memberships m
      WHERE m.user_id = u.id
        AND m.company_id = c.id
  );

INSERT INTO countries (name, code)
SELECT 'Brazil', 'BR'
WHERE NOT EXISTS (SELECT 1 FROM countries WHERE code = 'BR');

INSERT INTO banks (name, code, country_id)
SELECT 'Nubank', '260', c.id
FROM countries c
WHERE c.code = 'BR'
  AND NOT EXISTS (SELECT 1 FROM banks WHERE code = '260');

INSERT INTO banks (name, code, country_id)
SELECT 'C6 Bank', '336', c.id
FROM countries c
WHERE c.code = 'BR'
  AND NOT EXISTS (SELECT 1 FROM banks WHERE code = '336');
"#;

pub async fn run(db: &DatabaseConnection) -> doido::Result<()> {
    for statement in SEED_SQL.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        db.execute_unprepared(statement).await?;
    }

    Ok(())
}
