use std::fs;

use mpd24_ai_common::{ControllerProfile, ProfileVerification};
use mpd24_ai_midi_engine::profile::{
    load_profile, matches_device, profile_file_name, save_profile,
};
use tempfile::tempdir;

#[test]
fn mpd24_factory_profile_has_no_unverified_mappings() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../controller-profiles/akai-mpd24/profile.json");
    let profile = load_profile(&path).expect("load profile");
    assert_eq!(profile.verification, ProfileVerification::Unverified);
    assert!(profile.mappings.is_empty());
}

#[test]
fn device_matching_is_case_insensitive() {
    let profile = ControllerProfile::unverified("akai-mpd24", "Akai MPD24", vec!["MPD24".into()]);
    assert!(matches_device(&profile, "AKAI professional MPD24"));
    assert!(!matches_device(&profile, "Launchpad Pro"));
}

#[test]
fn profile_round_trips_to_json() {
    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("profile.json");
    let profile = ControllerProfile::unverified("akai-mpd24", "Akai MPD24", vec!["MPD24".into()]);
    save_profile(&path, &profile).expect("save");
    let loaded = load_profile(&path).expect("load");
    assert_eq!(loaded, profile);
    assert!(fs::metadata(path).is_ok());
}

#[test]
fn profile_file_name_rejects_path_traversal() {
    assert_eq!(
        profile_file_name("akai-mpd24").expect("valid id"),
        "akai-mpd24.json"
    );
    assert!(profile_file_name("../outside").is_err());
    assert!(profile_file_name("folder/profile").is_err());
    assert!(profile_file_name("").is_err());
}
