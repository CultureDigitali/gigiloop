#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod sequencer_runtime;
mod state;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_midi_inputs,
            commands::connect_midi_input,
            commands::disconnect_midi_input,
            commands::save_controller_profile,
            commands::ensure_audio_engine,
            commands::trigger_virtual_pad,
            commands::set_master_gain,
            commands::get_kit_state,
            commands::list_sample_library,
            commands::pick_and_import_pad_sample,
            commands::assign_library_sample,
            commands::set_pad_mixer,
            commands::set_pad_playback,
            commands::set_pad_velocity_layers,
            commands::reset_pad_to_demo,
            commands::get_pattern,
            commands::get_pattern_bank,
            commands::get_pattern_bank_status,
            commands::launch_pattern_slot,
            commands::copy_active_pattern_to_slot,
            commands::list_projects,
            commands::save_project,
            commands::load_project,
            commands::delete_project,
            commands::update_pattern_step,
            commands::set_pattern_length,
            commands::set_sequencer_bpm,
            commands::set_sequencer_swing,
            commands::clear_pattern,
            commands::load_demo_pattern,
            commands::start_sequencer,
            commands::stop_sequencer,
            commands::set_sequencer_recording,
            commands::generate_groove_candidate,
            commands::generate_hybrid_groove_candidate,
            commands::generate_directed_groove_candidate,
            commands::get_jev_status,
            commands::accept_groove_candidate,
            commands::discard_groove_candidate,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MPD24-AI");
}
