#!/usr/bin/env python3
"""Parent-run, bounded repeated observations; never adopts a runtime change."""
import csv
import json
import os
import pathlib
import signal
import statistics
import subprocess
import time

root = pathlib.Path(__file__).resolve().parent
binaries = {mode:root/'results'/f'issue3-probe-{mode}' for mode in ['plain','alloc']}
out = root / 'results'
out.mkdir(exist_ok=True)
start = time.monotonic()
rows = []
for repeat in range(7):
    order = ['current', 'candidate', 'sorted']
    offset = repeat % len(order)
    order = order[offset:] + order[:offset]
    if repeat % 2: order.reverse()
    for mode, variant in [(mode, variant) for variant in order for mode in ['plain','alloc']]:
        remaining = 300 - (time.monotonic() - start)
        if remaining <= 0: raise SystemExit('bounded measurement deadline exhausted; no decision evidence')
        stem = out / f'repeat-{repeat}-{variant}-{mode}'
        expected_fixtures = {f'{bound}-{shape}' for bound in ['representative','maximum'] for shape in ['sequential','random','mixed','prefix']}
        expected_phases = {'validate_prior','decode_next_blocks','index_edit_convert','snapshot_clone','entry_clone','sorted_entry_iteration','canonical_full_build','plan_edits','diff_maps','pipeline'}
        seen = set()
        with stem.with_suffix('.tsv').open('w') as stdout, stem.with_suffix('.stderr').open('w') as stderr:
            process = subprocess.Popen([str(binaries[mode]), 'measure', variant], stdout=stdout, stderr=stderr, start_new_session=True)
            def deadline(signum, frame):
                raise TimeoutError('bounded measurement child deadline exhausted')
            previous = signal.signal(signal.SIGALRM, deadline)
            signal.setitimer(signal.ITIMER_REAL, remaining)
            try:
                _, status, usage = os.wait4(process.pid, 0)
                process.returncode = os.waitstatus_to_exitcode(status)
                if process.returncode:
                    raise subprocess.CalledProcessError(process.returncode, process.args)
            except BaseException:
                if process.returncode is None:
                    os.killpg(process.pid, signal.SIGKILL)
                    _, status = os.waitpid(process.pid, 0)
                    process.returncode = os.waitstatus_to_exitcode(status)
                raise
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
                signal.signal(signal.SIGALRM, previous)
            stem.with_suffix('.usage.json').write_text(json.dumps({'max_rss_kib':usage.ru_maxrss,
                'user_seconds':usage.ru_utime,'system_seconds':usage.ru_stime,
                'minor_faults':usage.ru_minflt,'major_faults':usage.ru_majflt},indent=2)+'\n')
        with stem.with_suffix('.tsv').open() as stream:
            for row in csv.DictReader(stream, delimiter='\t'):
                row['repeat'] = repeat
                row['mode'] = mode
                assert row['variant'] == variant, row
                key = (row['fixture'],row['phase'])
                assert key not in seen and key[0] in expected_fixtures and key[1] in expected_phases, row
                seen.add(key)
                for key in row.keys() - {'variant','fixture','phase','repeat','mode'}: row[key] = int(row[key])
                if mode == 'plain':
                    assert all(row[field] == 0 for field in ['allocations','allocated_bytes','frees','peak_extra_live_bytes','retained_live_bytes']), ('unexpected timing instrumentation',row)
                elif row['phase'] == 'pipeline':
                    assert row['allocations'] > 0 and row['peak_extra_live_bytes'] > 0 and row['retained_live_bytes'] >= 0, ('allocation instrumentation missing or invalid',row)
                rows.append(row)
        assert seen == {(fixture,phase) for fixture in expected_fixtures for phase in expected_phases}, ('incomplete measurement output',stem,seen)

(out / 'all-observations.json').write_text(json.dumps(rows, indent=2) + '\n')
summary = {'schema':'issue3-observations-v1', 'runtime_adoption':False,
           'policy':'KEEP unless every gate below passes; success only permits a separately reviewed live integration experiment',
           'samples':7, 'semantic_evidence_required':'results/semantics.log and semantics-alloc.log with every named case and zero assertions/failures',
           'note':'Timing gates use the default System allocator without counting wrappers; allocation/live-byte gates use the separate allocation-counters binary. Both modes retain wait4 process RSS high-water marks in *.usage.json. Requested-live bytes exclude allocator overhead and realloc internal overlap. Standalone phases are not additive.',
           'comparisons':{}}
index = {(r['mode'],r['variant'],r['fixture'],r['phase'],r['repeat']):r for r in rows}
for variant in ['candidate','sorted']:
    details = []
    for fixture in sorted({r['fixture'] for r in rows}):
        base = [index[('plain','current',fixture,'pipeline',i)] for i in range(7)]
        test = [index[('plain',variant,fixture,'pipeline',i)] for i in range(7)]
        ratios = [t['ns']/b['ns'] for b,t in zip(base,test)]
        center_b = statistics.median(r['ns'] for r in base)
        center_t = statistics.median(r['ns'] for r in test)
        noise_b = statistics.median(abs(r['ns']-center_b) for r in base)/center_b
        noise_t = statistics.median(abs(r['ns']-center_t) for r in test)/center_t
        time_gate = statistics.median(ratios) <= 0.90 and sum(r <= 0.90 for r in ratios) >= 6 and max(noise_b,noise_t) <= 0.05
        memory_gate = True
        # Enforce the same <=5% bound for each phase, not only pipeline.
        for phase in sorted({r['phase'] for r in rows}):
            for i in range(7):
                b = index[('alloc','current',fixture,phase,i)]
                t = index[('alloc',variant,fixture,phase,i)]
                for field in ['peak_extra_live_bytes','rss_after_kib']:
                    memory_gate &= t[field] <= 1.05*b[field]
                # A zero retained baseline does not allow an invented 5% allowance.
                memory_gate &= max(0,t['retained_live_bytes']) <= 1.05*max(0,b['retained_live_bytes'])
                memory_gate &= max(0,t['rss_after_kib']-t['rss_before_kib']) <= 1.05*max(0,b['rss_after_kib']-b['rss_before_kib'])
        details.append({'fixture':fixture,'paired_time_ratios':ratios,'median_time_ratio':statistics.median(ratios),
                        'baseline_relative_mad':noise_b,'variant_relative_mad':noise_t,
                        'time_gate':time_gate,'requested_live_and_rss_gate':memory_gate})
    # The process max-RSS gate must also pass for every paired repeat.
    def maxrss(mode, variant, repeat):
        return json.loads((out / f'repeat-{repeat}-{variant}-{mode}.usage.json').read_text())['max_rss_kib']
    rss_gate = all(maxrss(mode,variant,i) <= 1.05*maxrss(mode,'current',i) for i in range(7) for mode in ['plain','alloc'])
    summary['comparisons'][variant] = {'fixtures':details,'process_peak_rss_gate':rss_gate,
                                     'measured_gates_pass':rss_gate and all(d['time_gate'] and d['requested_live_and_rss_gate'] for d in details),
                                     'decision':'KEEP pending parent review of semantic, platform, source and complete measurements'}
(out / 'summary.json').write_text(json.dumps(summary,indent=2)+'\n')
print(json.dumps(summary,indent=2))
