#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use mpd24_ai_groove_engine::GrooveParams;
use mpd24_ai_sequencer::{LaunchQuantization, PatternBank};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROJECT_SCHEMA_VERSION: u32 = 1;
pub const SCENE_SLOT_COUNT: usize = 8;
pub const KIT_PAD_COUNT: usize = 16;
pub const MAX_VELOCITY_LAYERS: usize = 8;
pub const MAX_ROUND_ROBIN_VARIANTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VelocityLayerSnapshot {
    pub min_velocity: u8,
    pub max_velocity: u8,
    pub sample_files: Vec<String>,
    pub round_robin: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KitPadSnapshot {
    pub display_name: String,
    pub sample_file: Option<String>,
    pub gain: f32,
    pub pan: f32,
    #[serde(default)]
    pub pitch_semitones: f32,
    #[serde(default)]
    pub reverse: bool,
    #[serde(default)]
    pub sample_start: f32,
    #[serde(default = "default_sample_end")]
    pub sample_end: f32,
    #[serde(default)]
    pub choke_group: Option<u8>,
    #[serde(default)]
    pub velocity_layers: Vec<VelocityLayerSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceScene {
    pub name: String,
    pub pattern_slot: usize,
    pub master_gain: f32,
    pub launch_quantization: LaunchQuantization,
    pub groove_params: GrooveParams,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub schema_version: u32,
    pub name: String,
    pub pattern_bank: PatternBank,
    pub launch_quantization: LaunchQuantization,
    pub master_gain: f32,
    pub groove_params: GrooveParams,
    #[serde(default = "empty_scenes")]
    pub scenes: Vec<Option<PerformanceScene>>,
    #[serde(default = "default_kit")]
    pub kit: Vec<KitPadSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: i64,
    pub name: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredProject {
    pub summary: ProjectSummary,
    pub snapshot: ProjectSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiFeedbackInput {
    pub project_id: Option<i64>,
    pub operation: String,
    pub provider: String,
    pub selected_id: String,
    pub local_score: f32,
    pub jev_confidence: Option<f32>,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OperationPreference {
    pub total: u64,
    pub accepted: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiPreferenceSummary {
    pub total: u64,
    pub accepted: u64,
    pub by_operation: BTreeMap<String, OperationPreference>,
}

#[derive(Debug, Error)]
pub enum ProjectStoreError {
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("project serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project name cannot be empty")]
    EmptyName,
    #[error("project not found: {0}")]
    NotFound(i64),
    #[error("unsupported project schema version: {0}")]
    UnsupportedSchema(u32),
    #[error("invalid pattern bank in project snapshot")]
    InvalidPatternBank,
    #[error("system clock is before Unix epoch")]
    Clock,
}

pub struct ProjectStore {
    connection: Connection,
}

impl ProjectStore {
    pub fn open(path: &Path) -> Result<Self, ProjectStoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, ProjectStoreError> {
        let connection = Connection::open_in_memory()?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn save(
        &self,
        id: Option<i64>,
        snapshot: &ProjectSnapshot,
    ) -> Result<ProjectSummary, ProjectStoreError> {
        validate_snapshot(snapshot)?;
        let now = now_ms()?;
        let json = serde_json::to_string(snapshot)?;

        match id {
            Some(id) => {
                let changed = self.connection.execute(
                    "UPDATE projects
                     SET name = ?1, snapshot_json = ?2, updated_at_ms = ?3
                     WHERE id = ?4",
                    params![snapshot.name, json, now, id],
                )?;
                if changed == 0 {
                    return Err(ProjectStoreError::NotFound(id));
                }
                self.summary(id)
            }
            None => {
                self.connection.execute(
                    "INSERT INTO projects(name, snapshot_json, created_at_ms, updated_at_ms)
                     VALUES (?1, ?2, ?3, ?3)",
                    params![snapshot.name, json, now],
                )?;
                self.summary(self.connection.last_insert_rowid())
            }
        }
    }

    pub fn load(&self, id: i64) -> Result<StoredProject, ProjectStoreError> {
        let row = self
            .connection
            .query_row(
                "SELECT name, snapshot_json, created_at_ms, updated_at_ms
                 FROM projects WHERE id = ?1",
                [id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or(ProjectStoreError::NotFound(id))?;
        let snapshot: ProjectSnapshot = serde_json::from_str(&row.1)?;
        validate_snapshot(&snapshot)?;
        Ok(StoredProject {
            summary: ProjectSummary {
                id,
                name: row.0,
                created_at_ms: row.2,
                updated_at_ms: row.3,
            },
            snapshot,
        })
    }

    pub fn list(&self) -> Result<Vec<ProjectSummary>, ProjectStoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, created_at_ms, updated_at_ms
             FROM projects ORDER BY updated_at_ms DESC, id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ProjectSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at_ms: row.get(2)?,
                updated_at_ms: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(ProjectStoreError::from)
    }

    pub fn delete(&self, id: i64) -> Result<(), ProjectStoreError> {
        let changed = self
            .connection
            .execute("DELETE FROM projects WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(ProjectStoreError::NotFound(id));
        }
        Ok(())
    }

    pub fn record_ai_feedback(&self, input: &AiFeedbackInput) -> Result<(), ProjectStoreError> {
        let now = now_ms()?;
        self.connection.execute(
            "INSERT INTO ai_feedback(
                project_id, operation, provider, selected_id, local_score,
                jev_confidence, accepted, created_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                input.project_id,
                input.operation,
                input.provider,
                input.selected_id,
                input.local_score,
                input.jev_confidence,
                input.accepted as i64,
                now
            ],
        )?;
        Ok(())
    }

    pub fn ai_preference_summary(
        &self,
        project_id: Option<i64>,
    ) -> Result<AiPreferenceSummary, ProjectStoreError> {
        let mut summary = AiPreferenceSummary::default();
        let mut statement = if project_id.is_some() {
            self.connection.prepare(
                "SELECT operation, accepted FROM ai_feedback
                 WHERE project_id = ?1 ORDER BY id ASC",
            )?
        } else {
            self.connection
                .prepare("SELECT operation, accepted FROM ai_feedback ORDER BY id ASC")?
        };

        let rows = if let Some(project_id) = project_id {
            statement.query_map([project_id], read_feedback_row)?
        } else {
            statement.query_map([], read_feedback_row)?
        };

        for row in rows {
            let (operation, accepted) = row?;
            summary.total += 1;
            if accepted {
                summary.accepted += 1;
            }
            let preference = summary.by_operation.entry(operation).or_default();
            preference.total += 1;
            if accepted {
                preference.accepted += 1;
            }
        }
        Ok(summary)
    }

    fn summary(&self, id: i64) -> Result<ProjectSummary, ProjectStoreError> {
        self.connection
            .query_row(
                "SELECT id, name, created_at_ms, updated_at_ms FROM projects WHERE id = ?1",
                [id],
                |row| {
                    Ok(ProjectSummary {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        created_at_ms: row.get(2)?,
                        updated_at_ms: row.get(3)?,
                    })
                },
            )
            .map_err(ProjectStoreError::from)
    }

    fn migrate(&self) -> Result<(), ProjectStoreError> {
        self.connection.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS projects (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 snapshot_json TEXT NOT NULL,
                 created_at_ms INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS ai_feedback (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 project_id INTEGER,
                 operation TEXT NOT NULL,
                 provider TEXT NOT NULL,
                 selected_id TEXT NOT NULL,
                 local_score REAL NOT NULL,
                 jev_confidence REAL,
                 accepted INTEGER NOT NULL,
                 created_at_ms INTEGER NOT NULL
             );",
        )?;
        Ok(())
    }
}

fn validate_snapshot(snapshot: &ProjectSnapshot) -> Result<(), ProjectStoreError> {
    if snapshot.name.trim().is_empty() {
        return Err(ProjectStoreError::EmptyName);
    }
    if snapshot.schema_version != PROJECT_SCHEMA_VERSION {
        return Err(ProjectStoreError::UnsupportedSchema(
            snapshot.schema_version,
        ));
    }
    if snapshot.pattern_bank.slots.len() != 8
        || snapshot.pattern_bank.active_slot >= snapshot.pattern_bank.slots.len()
    {
        return Err(ProjectStoreError::InvalidPatternBank);
    }
    if snapshot.scenes.len() != SCENE_SLOT_COUNT
        || snapshot
            .scenes
            .iter()
            .flatten()
            .any(|scene| scene.pattern_slot >= 8)
    {
        return Err(ProjectStoreError::InvalidPatternBank);
    }
    if snapshot.kit.len() != KIT_PAD_COUNT
        || snapshot.kit.iter().any(|pad| {
            !(0.0..=2.0).contains(&pad.gain)
                || !(-1.0..=1.0).contains(&pad.pan)
                || !(-24.0..=24.0).contains(&pad.pitch_semitones)
                || !(0.0..=0.999).contains(&pad.sample_start)
                || !(0.001..=1.0).contains(&pad.sample_end)
                || pad.sample_end <= pad.sample_start
                || pad.choke_group.is_some_and(|group| group == 0)
                || !valid_velocity_layers(&pad.velocity_layers)
        })
    {
        return Err(ProjectStoreError::InvalidPatternBank);
    }
    Ok(())
}

fn empty_scenes() -> Vec<Option<PerformanceScene>> {
    vec![None; SCENE_SLOT_COUNT]
}

pub fn default_kit() -> Vec<KitPadSnapshot> {
    (0..KIT_PAD_COUNT)
        .map(|index| KitPadSnapshot {
            display_name: format!("Demo Pad {}", index + 1),
            sample_file: None,
            gain: 1.0,
            pan: 0.0,
            pitch_semitones: 0.0,
            reverse: false,
            sample_start: 0.0,
            sample_end: 1.0,
            choke_group: matches!(index, 2 | 3 | 12).then_some(1),
            velocity_layers: Vec::new(),
        })
        .collect()
}

fn valid_velocity_layers(layers: &[VelocityLayerSnapshot]) -> bool {
    if layers.len() > MAX_VELOCITY_LAYERS {
        return false;
    }
    for (index, layer) in layers.iter().enumerate() {
        if layer.min_velocity == 0
            || layer.min_velocity > 127
            || layer.max_velocity > 127
            || layer.max_velocity < layer.min_velocity
            || layer.sample_files.is_empty()
            || layer.sample_files.len() > MAX_ROUND_ROBIN_VARIANTS
            || layer.sample_files.iter().any(|file| file.trim().is_empty())
            || layer
                .sample_files
                .iter()
                .enumerate()
                .any(|(index, file)| layer.sample_files[..index].contains(file))
        {
            return false;
        }
        for other in layers.iter().skip(index + 1) {
            let overlaps = layer.min_velocity <= other.max_velocity
                && other.min_velocity <= layer.max_velocity;
            if overlaps {
                return false;
            }
        }
    }
    true
}

fn default_sample_end() -> f32 {
    1.0
}

fn now_ms() -> Result<i64, ProjectStoreError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProjectStoreError::Clock)?;
    Ok(duration.as_millis().min(i64::MAX as u128) as i64)
}

fn read_feedback_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, bool)> {
    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
}
