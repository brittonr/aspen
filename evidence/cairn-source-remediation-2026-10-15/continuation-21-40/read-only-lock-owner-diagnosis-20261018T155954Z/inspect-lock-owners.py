"""Read-only, exact-owner inspection for the 2026-10-18 Nix diagnosis.

Run as root only after operator authentication. Writes JSON to stdout only.
Does not attach, signal, spawn, open FD targets, read environments or change state.
"""

import datetime
import json
import os
import sys

EXPECTED_BOOT_ID = '5b739154-922b-42bf-9559-d4c444746adb'
OWNERS = (
    {'pid': 92763, 'uid': 0, 'ppid': 2631, 'command': 'nix-daemon 92736',
     'start_ticks': 17230095, 'device': (0, 31), 'inode': 3940983},
    {'pid': 130832, 'uid': 0, 'ppid': 2631, 'command': 'nix-daemon 130700',
     'start_ticks': 17256828, 'device': (0, 31), 'inode': 4053942},
    {'pid': 2601744, 'uid': 0, 'ppid': 2631, 'command': 'nix-daemon 2601693',
     'start_ticks': 25667849, 'device': (0, 31), 'inode': 4482053},
    {'pid': 3432619, 'uid': 0, 'ppid': 2631, 'command': 'nix-daemon 3432598',
     'start_ticks': 21803025, 'device': (0, 31), 'inode': 4632022},
)
MAX_THREADS = 16
MAX_FDS = 64


def read_at(directory, name, limit=131072):
    fd = os.open(name, os.O_RDONLY | os.O_CLOEXEC, dir_fd=directory)
    try:
        chunks = []
        remaining = limit + 1
        while remaining:
            block = os.read(fd, min(remaining, 65536))
            if not block:
                break
            chunks.append(block)
            remaining -= len(block)
        data = b''.join(chunks)
        if len(data) > limit:
            raise RuntimeError(f'Read bound exceeded for {name}')
        return data.decode('utf-8', errors='replace')
    finally:
        os.close(fd)


def read_absolute(path, limit=131072):
    return read_at(None, path, limit)


def observe(directory, name, limit=131072):
    try:
        return {'value': read_at(directory, name, limit)}
    except (OSError, RuntimeError) as error:
        return {'error': str(error)}


def identity(directory):
    raw_stat = read_at(directory, 'stat')
    fields = raw_stat[raw_stat.rindex(')') + 2:].split()
    status = read_at(directory, 'status')
    uid_row = next(line for line in status.splitlines() if line.startswith('Uid:'))
    return {
        'pid': int(raw_stat.split(' ', 1)[0]),
        'uid': int(uid_row.split()[1]),
        'effective_uid': int(uid_row.split()[2]),
        'ppid': int(fields[1]),
        'start_ticks': int(fields[19]),
        'command': ' '.join(read_at(directory, 'cmdline').replace('\x00', ' ').split()),
    }


def held_lock_rows(expected):
    matches = []
    for line in read_absolute('/proc/locks', 16777216).splitlines():
        fields = line.split()
        if len(fields) != 8 or fields[1] != 'FLOCK' or fields[3] != 'WRITE':
            continue
        if int(fields[4]) != expected['pid']:
            continue
        major, minor, inode = fields[5].split(':')
        if ((int(major, 16), int(minor, 16)) == expected['device']
                and int(inode) == expected['inode']):
            matches.append(line)
    return matches


def verify(directory, expected):
    actual = identity(directory)
    for name in ('pid', 'uid', 'ppid', 'start_ticks', 'command'):
        if actual[name] != expected[name]:
            raise RuntimeError(f'Identity mismatch for {name}: {actual[name]!r}')
    if actual['effective_uid'] != expected['uid']:
        raise RuntimeError('Effective UID changed')
    lock_rows = held_lock_rows(expected)
    if not lock_rows:
        raise RuntimeError('Expected exact device/inode WRITE lock is not held by this PID')
    return {'identity': actual, 'held_lock_rows': lock_rows}


def inspect_owner(expected):
    result = {'expected': expected, 'status': 'skipped'}
    directory = None
    try:
        directory = os.open(f"/proc/{expected['pid']}", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
        before = verify(directory, expected)
        waits = {name: observe(directory, name) for name in ('wchan', 'syscall', 'stack')}
        threads = []
        task_directory = os.open('task', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC, dir_fd=directory)
        try:
            tids = sorted(os.listdir(task_directory), key=int)
            for tid in tids[:MAX_THREADS]:
                threads.append({'tid': int(tid), **{
                    name: observe(task_directory, f'{tid}/{name}')
                    for name in ('comm', 'wchan', 'syscall', 'stack')
                }})
        finally:
            os.close(task_directory)
        descriptors = []
        fd_directory = os.open('fd', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC, dir_fd=directory)
        try:
            fd_names = sorted(os.listdir(fd_directory), key=int)
            for name in fd_names[:MAX_FDS]:
                try:
                    target = {'value': os.readlink(name, dir_fd=fd_directory)}
                except OSError as error:
                    target = {'error': str(error)}
                descriptors.append({'fd': int(name), 'link_target': target,
                                    'fdinfo': observe(directory, f'fdinfo/{name}', 16384)})
        finally:
            os.close(fd_directory)
        after = verify(directory, expected)
        result.update(status='observed', before=before, after=after, waits=waits,
                      threads=threads, total_threads=len(tids), omitted_threads=max(0, len(tids) - MAX_THREADS),
                      descriptors=descriptors, total_fds=len(fd_names), omitted_fds=max(0, len(fd_names) - MAX_FDS))
    except (OSError, RuntimeError, ValueError, StopIteration) as error:
        result['reason'] = str(error)
        result['diagnostic_payload_discarded'] = True
    finally:
        if directory is not None:
            os.close(directory)
    return result


def main():
    report = {
        'recorded_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
        'read_only': True,
        'scope': 'Four fixed current-lock-owner candidates only; no attachment or process control',
        'limits': ['Snapshots are not atomic, even with before/after identity and lock guards',
                   'Descriptors are readlink/fdinfo metadata only; their targets are not opened',
                   'No authority to cancel any worker or claim acceptance from these observations'],
    }
    if os.geteuid() != 0:
        report.update(status='requires-operator-root-authentication', owners=[])
        print(json.dumps(report, indent=2))
        return 2
    boot_id = read_absolute('/proc/sys/kernel/random/boot_id').strip()
    if boot_id != EXPECTED_BOOT_ID:
        report.update(status='skipped-host-boot-changed', owners=[])
        print(json.dumps(report, indent=2))
        return 3
    report.update(status='read-only-snapshot-complete', owners=[inspect_owner(owner) for owner in OWNERS])
    print(json.dumps(report, indent=2))
    return 0 if all(owner['status'] == 'observed' for owner in report['owners']) else 4


if __name__ == '__main__':
    sys.exit(main())
