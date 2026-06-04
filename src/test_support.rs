use std::fs;
use std::path::Path;
use std::sync::Once;

static CLEAN_STALE_TEMP_DIRS: Once = Once::new();

pub(crate) fn cleanup_stale_molten_temp_dirs() {
    CLEAN_STALE_TEMP_DIRS.call_once(|| {
        let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
            return;
        };
        for entry_result in entries {
            let Ok(entry) = entry_result else {
                continue;
            };
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                let file_name = entry.file_name();
                let Some(name) = file_name.to_str() else {
                    continue;
                };
                if is_stale_molten_temp_dir(name) {
                    let remove_result = fs::remove_dir_all(entry.path());
                    if remove_result.is_err() {
                        continue;
                    }
                }
            }
        }
    });
}

fn is_stale_molten_temp_dir(name: &str) -> bool {
    name.starts_with("molten-") && live_process_token_count(name) == 0
}

fn live_process_token_count(name: &str) -> usize {
    let current_pid = u64::from(std::process::id());
    name.split('-')
        .filter_map(|token| token.parse::<u64>().ok())
        .filter(|pid| *pid == current_pid || process_is_alive(*pid))
        .count()
}

fn process_is_alive(pid: u64) -> bool {
    if pid == 0 {
        return false;
    }
    Path::new("/proc").join(pid.to_string()).exists()
}
