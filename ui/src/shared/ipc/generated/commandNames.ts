// Gerado por `cargo test -p sofa --test bindings` a partir de src-tauri/command_list.rs.
// Nao edite a mao.

export const COMMAND_NAMES = [
  'library_get',
  'library_refresh',
  'library_add_manual',
  'library_hidden',
  'game_rename',
  'game_restore_name',
  'game_hide',
  'game_unhide',
  'fs_list_dir',
  'session_get',
  'session_launch',
  'session_launch_with_uac',
  'session_replace',
  'session_return_to_game',
  'session_close',
  'session_force_close',
  'session_retry',
  'session_dismiss',
  'volume_get',
  'volume_step',
  'volume_toggle_mute',
  'power_capabilities',
  'power_prepare',
  'power_execute',
  'settings_get',
  'settings_set_autostart',
  'settings_set_monitor',
  'display_monitors',
  'input_status',
  'app_exit',
] as const;

export type CommandName = (typeof COMMAND_NAMES)[number];
