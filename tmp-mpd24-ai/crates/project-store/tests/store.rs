use mpd24_ai_groove_engine::GrooveParams;
use mpd24_ai_project_store::{
    default_kit, AiFeedbackInput, KitPadSnapshot, ProjectSnapshot, ProjectStore, ProjectStoreError,
    VelocityLayerSnapshot, PROJECT_SCHEMA_VERSION,
};
use mpd24_ai_sequencer::{LaunchQuantization, PatternBank};
use serde_json::json;

fn snapshot(name: &str) -> ProjectSnapshot {
    ProjectSnapshot {
        schema_version: PROJECT_SCHEMA_VERSION,
        name: name.into(),
        pattern_bank: PatternBank::default(),
        launch_quantization: LaunchQuantization::NextBar,
        master_gain: 0.86,
        groove_params: GrooveParams::default(),
        scenes: vec![None; 8],
        kit: default_kit(),
    }
}

#[test]
fn saves_lists_and_loads_project_round_trip() {
    let store = ProjectStore::open_in_memory().expect("store");
    let saved = store.save(None, &snapshot("First Beat")).expect("save");
    let listed = store.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "First Beat");

    let loaded = store.load(saved.id).expect("load");
    assert_eq!(loaded.snapshot, snapshot("First Beat"));
}

#[test]
fn updating_existing_project_keeps_id() {
    let store = ProjectStore::open_in_memory().expect("store");
    let first = store.save(None, &snapshot("Draft")).expect("save");
    let mut updated = snapshot("Final");
    updated.master_gain = 0.5;
    let second = store.save(Some(first.id), &updated).expect("update");
    assert_eq!(first.id, second.id);
    assert_eq!(store.load(first.id).expect("load").snapshot, updated);
}

#[test]
fn delete_removes_project() {
    let store = ProjectStore::open_in_memory().expect("store");
    let saved = store.save(None, &snapshot("Delete me")).expect("save");
    store.delete(saved.id).expect("delete");
    assert!(matches!(
        store.load(saved.id),
        Err(ProjectStoreError::NotFound(id)) if id == saved.id
    ));
}

#[test]
fn rejects_empty_project_names() {
    let store = ProjectStore::open_in_memory().expect("store");
    assert!(matches!(
        store.save(None, &snapshot("  ")),
        Err(ProjectStoreError::EmptyName)
    ));
}

#[test]
fn records_ai_feedback_and_builds_preference_summary() {
    let store = ProjectStore::open_in_memory().expect("store");
    for (operation, accepted) in [("variation", true), ("variation", false), ("fill", true)] {
        store
            .record_ai_feedback(&AiFeedbackInput {
                project_id: None,
                operation: operation.into(),
                provider: "jev".into(),
                selected_id: "candidate-1".into(),
                local_score: 0.7,
                jev_confidence: Some(0.8),
                accepted,
            })
            .expect("feedback");
    }
    let summary = store.ai_preference_summary(None).expect("summary");
    assert_eq!(summary.total, 3);
    assert_eq!(summary.accepted, 2);
    assert_eq!(summary.by_operation["variation"].total, 2);
    assert_eq!(summary.by_operation["variation"].accepted, 1);
    assert_eq!(summary.by_operation["fill"].accepted, 1);
}

#[test]
fn legacy_kit_json_receives_advanced_sampler_defaults() {
    let legacy = json!({
        "displayName": "Legacy Kick",
        "sampleFile": "kick.wav",
        "gain": 0.8,
        "pan": -0.1
    });
    let pad: KitPadSnapshot = serde_json::from_value(legacy).expect("legacy kit pad");
    assert_eq!(pad.pitch_semitones, 0.0);
    assert!(!pad.reverse);
    assert_eq!(pad.sample_start, 0.0);
    assert_eq!(pad.sample_end, 1.0);
    assert_eq!(pad.choke_group, None);
    assert!(pad.velocity_layers.is_empty());
}

#[test]
fn default_hat_pads_keep_choke_group_one() {
    let kit = default_kit();
    assert_eq!(kit[2].choke_group, Some(1));
    assert_eq!(kit[3].choke_group, Some(1));
    assert_eq!(kit[12].choke_group, Some(1));
    assert_eq!(kit[0].choke_group, None);
}

#[test]
fn project_round_trip_preserves_velocity_layers() {
    let store = ProjectStore::open_in_memory().expect("store");
    let mut project = snapshot("Layered");
    project.kit[1].velocity_layers = vec![
        VelocityLayerSnapshot {
            min_velocity: 1,
            max_velocity: 63,
            sample_files: vec!["snare-soft-a.wav".into(), "snare-soft-b.wav".into()],
            round_robin: true,
        },
        VelocityLayerSnapshot {
            min_velocity: 64,
            max_velocity: 127,
            sample_files: vec!["snare-hard.wav".into()],
            round_robin: false,
        },
    ];
    let saved = store.save(None, &project).expect("save");
    assert_eq!(store.load(saved.id).expect("load").snapshot, project);
}

#[test]
fn rejects_overlapping_velocity_layers() {
    let store = ProjectStore::open_in_memory().expect("store");
    let mut project = snapshot("Overlap");
    project.kit[0].velocity_layers = vec![
        VelocityLayerSnapshot {
            min_velocity: 1,
            max_velocity: 80,
            sample_files: vec!["a.wav".into()],
            round_robin: false,
        },
        VelocityLayerSnapshot {
            min_velocity: 70,
            max_velocity: 127,
            sample_files: vec!["b.wav".into()],
            round_robin: false,
        },
    ];
    assert!(matches!(
        store.save(None, &project),
        Err(ProjectStoreError::InvalidPatternBank)
    ));
}

#[test]
fn rejects_duplicate_round_robin_variants() {
    let store = ProjectStore::open_in_memory().expect("store");
    let mut project = snapshot("Duplicate RR");
    project.kit[0].velocity_layers = vec![VelocityLayerSnapshot {
        min_velocity: 1,
        max_velocity: 127,
        sample_files: vec!["kick-a.wav".into(), "kick-a.wav".into()],
        round_robin: true,
    }];
    assert!(matches!(
        store.save(None, &project),
        Err(ProjectStoreError::InvalidPatternBank)
    ));
}
