"""Bounded, read-only GC-lock discovery independent of /proc/locks rows.

Only FD metadata matching the fixed GC device/inode is retained. Wait-state
reads are limited to processes with a revalidated held lock on that inode.
No attachment, signals, environment reads, FD-target opens or host writes.
"""

import collections
import datetime
import errno
import json
import os
import sys
import time

BOOT_ID = '5b739154-922b-42bf-9559-d4c444746adb'
GC_PATH = '/nix/var/nix/gc.lock'
GC_TARGET = (os.makedev(0, 31), 22402385)
MAX_PROCESSES = 16384
MAX_DESCRIPTORS = 262144
MAX_THREADS = 16


def read_at(directory, name, limit=131072):
    fd = os.open(name, os.O_RDONLY | os.O_CLOEXEC, dir_fd=directory)
    try:
        chunks = []
        remaining = limit + 1
        while remaining:
            chunk = os.read(fd, min(remaining, 65536))
            if not chunk:
                break
            chunks.append(chunk)
            remaining -= len(chunk)
        data = b''.join(chunks)
        if len(data) > limit:
            raise RuntimeError(f'Read bound exceeded: {name}')
        return data.decode('utf-8', errors='replace')
    finally:
        os.close(fd)


def identity(proc):
    stat = read_at(proc, 'stat')
    fields = stat[stat.rindex(')') + 2:].split()
    uid = next(line.split()[1:] for line in read_at(proc, 'status').splitlines()
               if line.startswith('Uid:'))
    return {'pid': int(stat.split(' ', 1)[0]), 'ppid': int(fields[1]),
            'start_ticks': int(fields[19]), 'uid': int(uid[0]), 'effective_uid': int(uid[1]),
            'command': ' '.join(read_at(proc, 'cmdline').replace('\x00', ' ').split())}


def held_rows(text, target):
    rows = []
    for line in text.splitlines():
        if not line.startswith('lock:'):
            continue
        fields = line.split()[1:]
        if len(fields) != 8 or fields[1] != 'FLOCK' or fields[3] not in ('READ', 'WRITE'):
            continue
        major, minor, inode = fields[5].split(':')
        if (os.makedev(int(major, 16), int(minor, 16)), int(inode)) == target:
            rows.append(line)
    return rows


def inspect_fd(proc, descriptors, name, target):
    before = os.stat(name, dir_fd=descriptors)
    if (before.st_dev, before.st_ino) != target:
        return None
    info = read_at(proc, f'fdinfo/{name}', 16384)
    after = os.stat(name, dir_fd=descriptors)
    if (after.st_dev, after.st_ino) != target:
        return None
    return {'fd': int(name), 'fdinfo': info, 'held_lock_rows': held_rows(info, target)}


def observe(proc, name):
    try:
        return {'value': read_at(proc, name)}
    except (OSError, RuntimeError) as error:
        return {'error': str(error)}


def wait_snapshot(proc, deadline):
    result = {'main': {}, 'threads': [], 'thread_limit_reached': False}
    for name in ('wchan', 'syscall', 'stack'):
        if time.monotonic() >= deadline:
            result['deadline_reached'] = True
            return result
        result['main'][name] = observe(proc, name)
    tasks = os.open('task', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC, dir_fd=proc)
    try:
        with os.scandir(tasks) as entries:
            for entry in entries:
                if not entry.name.isdecimal():
                    continue
                if len(result['threads']) >= MAX_THREADS:
                    result['thread_limit_reached'] = True
                    break
                if time.monotonic() >= deadline:
                    result['deadline_reached'] = True
                    break
                result['threads'].append({'tid': int(entry.name), **{
                    name: observe(tasks, f'{entry.name}/{name}')
                    for name in ('comm', 'wchan', 'syscall', 'stack')
                }})
    finally:
        os.close(tasks)
    return result


def inspect_process(pid, target, deadline, counts):
    proc = os.open(f'/proc/{pid}', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        before = identity(proc)
        candidates = []
        descriptors = os.open('fd', os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC, dir_fd=proc)
        try:
            with os.scandir(descriptors) as entries:
                for entry in entries:
                    if not entry.name.isdecimal():
                        continue
                    if time.monotonic() >= deadline or counts['descriptors_examined'] >= MAX_DESCRIPTORS:
                        counts['scan_limit_reached'] = 1
                        break
                    counts['descriptors_examined'] += 1
                    try:
                        fd = inspect_fd(proc, descriptors, entry.name, target)
                        if fd is not None:
                            counts['gc_descriptors_seen'] += 1
                            if fd['held_lock_rows']:
                                candidates.append(fd)
                    except OSError as error:
                        counts['fd_disappeared' if error.errno in (errno.ENOENT, errno.ESRCH) else 'fd_read_errors'] += 1
                    except (ValueError, RuntimeError):
                        counts['fd_parse_errors'] += 1
            if not candidates:
                return None
            if identity(proc) != before:
                return {'status': 'skipped-identity-changed', 'pid': pid}
            stable = [inspect_fd(proc, descriptors, str(fd['fd']), target) for fd in candidates]
            if any(fd is None or not fd['held_lock_rows'] for fd in stable):
                return {'status': 'skipped-lock-changed', 'pid': pid}
            waits = wait_snapshot(proc, deadline)
            after = identity(proc)
            final = [inspect_fd(proc, descriptors, str(fd['fd']), target) for fd in stable]
            if after != before or any(fd is None or not fd['held_lock_rows'] for fd in final):
                return {'status': 'skipped-owner-or-lock-changed', 'pid': pid}
            return {'status': 'observed', 'before': before, 'after': after,
                    'held_descriptors_before': stable, 'held_descriptors_after': final, 'waits': waits}
        finally:
            os.close(descriptors)
    finally:
        os.close(proc)


def main():
    report = {'recorded_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
              'read_only': True, 'target': {'path': GC_PATH, 'device': '0:31', 'inode': GC_TARGET[1]},
              'limits': ['PID namespace visibility can still limit which processes are enumerable',
                         'Before/after metadata checks are not atomic',
                         'No observed holder or incomplete scan is not proof of recovery',
                         'A descriptor holder is not necessarily the original lock-acquiring PID',
                         'No cancellation, lock removal or acceptance authority is conferred']}
    if os.geteuid() != 0:
        report['status'] = 'requires-operator-root-authentication'
        print(json.dumps(report, indent=2))
        return 2
    target_stat = os.stat(GC_PATH)
    if (read_at(None, '/proc/sys/kernel/random/boot_id').strip() != BOOT_ID
            or (target_stat.st_dev, target_stat.st_ino) != GC_TARGET):
        report['status'] = 'skipped-boot-or-gc-inode-changed'
        print(json.dumps(report, indent=2))
        return 3
    deadline = time.monotonic() + 15
    counts = collections.Counter()
    holders = []
    with os.scandir('/proc') as processes:
        for process in processes:
            if not process.name.isdecimal():
                continue
            if (time.monotonic() >= deadline or counts['processes_examined'] >= MAX_PROCESSES
                    or counts['scan_limit_reached']):
                counts['scan_limit_reached'] = 1
                break
            counts['processes_examined'] += 1
            try:
                holder = inspect_process(int(process.name), GC_TARGET, deadline, counts)
                if holder is not None:
                    holders.append(holder)
            except OSError as error:
                counts['process_disappeared' if error.errno in (errno.ENOENT, errno.ESRCH) else 'process_read_errors'] += 1
            except (RuntimeError, ValueError, StopIteration):
                counts['process_parse_errors'] += 1
    report.update(status='bounded-read-only-scan-finished', counts=dict(counts), holders=holders,
                  scan_limit_reached=bool(counts['scan_limit_reached']))
    print(json.dumps(report, indent=2))
    return 0


if __name__ == '__main__':
    sys.exit(main())
