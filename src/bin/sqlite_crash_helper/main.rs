/// Helper binary for testing SQLite crash recovery.
///
/// This binary is spawned as a subprocess to write data to SQLite
/// and then exit ungracefully (without closing connections or flushing).
use std::env;
use std::process;

use aspen::raft::storage_sqlite::SqliteStateMachine;
use aspen::raft::types::AppRequest;
use aspen::raft::types::AppTypeConfig;
use aspen::raft::types::NodeId;
use futures::stream::{self};
use openraft::entry::RaftEntry;
use openraft::storage::RaftStateMachine;
use openraft::testing::log_id;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <db_path> <num_entries>", args[0]);
        process::exit(1);
    }

    let db_path = &args[1];
    let num_entries: usize = args[2].parse().expect("Invalid number");

    // Open SQLite database
    let mut sm = SqliteStateMachine::new(db_path).expect("Failed to create state machine");

    // Write entries
    for i in 0..num_entries {
        let entry = <AppTypeConfig as openraft::RaftTypeConfig>::Entry::new_normal(
            log_id::<AppTypeConfig>(1, NodeId::from(1), i as u64),
            AppRequest::Set {
                key: format!("crash_key_{}", i),
                value: format!("crash_value_{}", i),
            },
        );

        let entries = Box::pin(stream::once(async move { Ok((entry, None)) }));
        sm.apply(entries).await.expect("Failed to apply entry");

        // Print progress for debugging
        if (i + 1) % 10 == 0 {
            println!("Wrote {} entries", i + 1);
        }
    }

    println!("Successfully wrote {} entries", num_entries);

    // Now simulate a crash by calling abort()
    // This exits immediately without running destructors or cleanup
    // SAFETY: abort() is a POSIX syscall that terminates the process immediately.
    // This is intentional for testing SQLite's crash recovery. No cleanup is needed
    // since the test specifically validates durability after an unclean shutdown.
    unsafe {
        libc::abort();
    }
}
