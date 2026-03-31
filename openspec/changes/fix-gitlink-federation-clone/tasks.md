## 1. TreeEntry gitlink support

- [x] 1.1 Add `gitlink()` constructor to `TreeEntry` that stores SHA-1 zero-padded to 32 bytes with mode 160000
- [x] 1.2 Add `is_gitlink()` method returning true for mode 160000
- [x] 1.3 Add `gitlink_sha1_bytes()` method extracting first 20 bytes from hash field

## 2. Tree import fix

- [x] 2.1 Change `parse_git_tree_content` to preserve gitlink entries using `TreeEntry::gitlink()` instead of `continue`
- [x] 2.2 Verify tree entry sort order is unchanged (gitlinks sort like regular files, not directories)

## 3. Tree export fix

- [x] 3.1 Update `export_tree` to check `is_gitlink()` and write SHA-1 from `gitlink_sha1_bytes()` directly
- [x] 3.2 Bypass BLAKE3→SHA-1 mapping lookup for gitlink entries

## 4. DAG walk fix

- [x] 4.1 Filter gitlink entries in `export_commit_dag_queue_deps` (used by SHA-1 based fetch)
- [x] 4.2 Filter gitlink entries in `get_object_deps` (used by BLAKE3 based federation export)

## 5. Testing

- [x] 5.1 Verify existing forge tests pass (363 tests)
- [x] 5.2 Run federation dogfood end-to-end: push 34K objects to alice, federated clone through bob
- [x] 5.3 Add unit test for gitlink tree import/export round-trip
- [x] 5.4 Add unit test verifying DAG walk skips gitlink entries
