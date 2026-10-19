#!/usr/bin/env python3
"""Bounded long-window command recorder.

Streams the child's combined output to a log file, enforces one absolute
deadline, terminates only its own process group on expiry, and records exit
status, timing, and a bounded tail of the log.

Usage: record-long-command.py OUTPUT_JSON LOG_PATH DEADLINE_SECONDS CMD [ARG...]
"""

import datetime
import json
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path

TAIL_BYTES = 262144
LOG_CAP_BYTES = 512 * 1024 * 1024


def main():
    output, logpath, deadline_text, *argv = sys.argv[1:]
    deadline = float(deadline_text)
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    before = time.monotonic()
    child = subprocess.Popen(
        argv,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        bufsize=0,
        start_new_session=True,
    )
    state = {'total': 0, 'truncated': False}

    with open(logpath, 'wb') as log:
        def pump():
            while True:
                chunk = child.stdout.read(65536)
                if not chunk:
                    break
                state['total'] += len(chunk)
                if state['total'] <= LOG_CAP_BYTES:
                    log.write(chunk)
                    log.flush()
                else:
                    state['truncated'] = True
            child.stdout.close()

        thread = threading.Thread(target=pump, daemon=True)
        thread.start()

        timed_out = False
        try:
            exit_code = child.wait(timeout=deadline)
        except subprocess.TimeoutExpired:
            timed_out = True
            os.killpg(child.pid, signal.SIGTERM)
            try:
                exit_code = child.wait(timeout=30)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                exit_code = child.wait()
        thread.join(timeout=60)

    size = 0
    tail = b''
    try:
        with open(logpath, 'rb') as log:
            log.seek(0, os.SEEK_END)
            size = log.tell()
            log.seek(max(0, size - TAIL_BYTES))
            tail = log.read()
    except OSError:
        pass

    result = {
        'argv': argv,
        'cwd': os.getcwd(),
        'started_at': started,
        'elapsed_seconds': round(time.monotonic() - before, 3),
        'deadline_seconds': deadline,
        'exit_code': exit_code,
        'timed_out': timed_out,
        'log_path': logpath,
        'log_bytes_streamed': state['total'],
        'log_file_bytes': size,
        'log_truncated_at_bytes': LOG_CAP_BYTES if state['truncated'] else None,
        'tail': tail.decode('utf-8', 'replace'),
        'environment': {
            'IN_NIX_SHELL': os.environ.get('IN_NIX_SHELL'),
            'CARGO_TARGET_DIR': os.environ.get('CARGO_TARGET_DIR'),
        },
    }
    Path(output).write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({
        'record': output,
        'log': logpath,
        'exit_code': exit_code,
        'timed_out': timed_out,
        'seconds': result['elapsed_seconds'],
        'log_bytes_streamed': state['total'],
    }))
    return 0 if exit_code == 0 and not timed_out else 1


if __name__ == '__main__':
    raise SystemExit(main())
