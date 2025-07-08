#!/usr/bin/env python3
"""
Script to analyze all .ser examples and generate a serializability report (optimized only)
Usage: python3 analyze_examples.py [--timeout <seconds>] [--jobs <number>] [--use-cache]
                                  [--without-remove-redundant] [--without-generate-less]
                                  [--without-smart-kleene-order] [--without-bidirectional]
                                  [--<other_flags>]
"""

import argparse
import subprocess
import time
import os
import re
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime


def parse_time_output(stderr_output):
    """Parse time command output to extract user and sys time."""
    lines = stderr_output.strip().split('\n')
    user_time = sys_time = 0.0
    for line in lines:
        if 'user' in line:
            m = re.search(r'(\d+)m([\d.]+)s', line)
            if m:
                user_time = int(m.group(1)) * 60 + float(m.group(2))
            else:
                m = re.search(r'([\d.]+)', line)
                if m:
                    user_time = float(m.group(1))
        elif 'sys' in line:
            m = re.search(r'(\d+)m([\d.]+)s', line)
            if m:
                sys_time = int(m.group(1)) * 60 + float(m.group(2))
            else:
                m = re.search(r'([\d.]+)', line)
                if m:
                    sys_time = float(m.group(1))
    return user_time + sys_time


def run_single_analysis(file_path, timeout_arg, extra_flags, use_cache=False):
    """Run a single optimized analysis with optional extra flags."""
    cmd = ['cargo', 'run', '--quiet', '--'] + extra_flags
    if timeout_arg:
        cmd += ['--timeout', str(timeout_arg)]
    if use_cache:
        cmd.append('--use-cache')
    cmd.append(str(file_path))

    time_cmd = ['time'] + cmd
    start = time.time()
    try:
        result = subprocess.run(
            time_cmd,
            capture_output=True,
            text=True,
            timeout=(timeout_arg * 2) if timeout_arg else None
        )
    except subprocess.TimeoutExpired:
        return {
            'status': '⏱️ SMPT Timeout',
            'original_result': 'SMPT Timeout',
            'proof_result': 'SMPT Timeout',
            'trace_valid': None,
            'proof_verification': None,
            'cpu_time': 0.0,
            'is_timeout': True
        }

    cpu = parse_time_output(result.stderr)
    if cpu == 0.0:
        cpu = time.time() - start

    out = result.stdout + result.stderr
    timeout_flag = 'SMPT timeout:' in out or 'Analysis timed out' in out or '⏱️ RESULT: TIMEOUT' in out or 'Number of components in semilinear set is too large' in out

    orig = 'Unknown'
    proof = 'Unknown'
    trace_valid = None
    proof_valid = None
    if result.returncode == 0:
        # Check for the new output format
        if '✅ RESULT: SERIALIZABLE' in out:
            orig = 'Serializable'
            proof = 'Serializable'
            # Check for proof certificate validity
            if '✅ Proof certificate is VALID' in out:
                proof_valid = True
            elif '❌ Proof certificate is INVALID' in out:
                proof_valid = False
            elif '✅ PROOF CERTIFICATE FOUND' in out:
                proof_valid = True
        elif '❌ RESULT: NOT SERIALIZABLE' in out:
            orig = 'Not serializable'
            proof = 'Not serializable'
            # Check for trace validity
            if '✅ Trace is valid!' in out:
                trace_valid = True
            elif '❌ Trace is INVALID!' in out:
                trace_valid = False
            elif '❌ COUNTEREXAMPLE TRACE FOUND' in out:
                trace_valid = True
        elif '⏱️ RESULT: TIMEOUT' in out:
            orig = 'SMPT Timeout'
            proof = 'SMPT Timeout'
    else:
        if timeout_flag:
            orig = proof = 'SMPT Timeout'
        else:
            orig = proof = 'Error'

    if orig == proof:
        if orig == 'Serializable':
            status = '✅ Serializable'
        elif orig == 'Not serializable':
            status = '❌ Not serializable'
        elif orig == 'SMPT Timeout':
            status = '⏱️ SMPT Timeout'
        else:
            status = '⚠️ Error'
    else:
        status = '⚠️ Error'

    return {
        'status': status,
        'original_result': orig,
        'proof_result': proof,
        'trace_valid': trace_valid,
        'proof_verification': proof_valid,
        'cpu_time': cpu,
        'is_timeout': timeout_flag
    }


def analyze_file(fp, timeout, idx, extra_flags, use_cache):
    name = Path(fp).stem
    print(f"[{idx}] `{name}`: Running optimized analysis...")
    res = run_single_analysis(fp, timeout, extra_flags, use_cache)
    dur = f"{res['cpu_time']:.2f}"
    print(f"[{idx}] `{name}`: {res['status']} ({dur}s CPU)")
    res.update({'filename': name, 'duration': dur, 'index': idx})
    return res


def run_analysis(files, timeout, jobs, cache, extras, suffix=""):
    """Run analysis on files with given options."""
    print(f"🔍 Analyzing (.ser & .json) with {jobs} jobs, timeout={timeout or 'none'}, "
          f"cache={'on' if cache else 'off'}, extras={extras}")
    print(f"Found {len(files)} examples")

    results = []
    with ThreadPoolExecutor(max_workers=jobs) as ex:
        futures = {ex.submit(analyze_file, fp, timeout, i, extras, cache): i for i, fp in enumerate(files)}
        for fut in as_completed(futures):
            results.append(fut.result())
    results.sort(key=lambda r: r['index'])

    # Print non-validated cases
    nv_proofs = [r['filename'] for r in results
                 if r['original_result'] == 'Serializable'
                 and r['proof_result'] == 'Serializable'
                 and r['proof_verification'] is False]
    nv_traces = [r['filename'] for r in results
                 if r['original_result'] == 'Not serializable'
                 and r['proof_result'] == 'Not serializable'
                 and r['trace_valid'] is False]
    if nv_proofs:
        print('❌ Non-validated proofs for:', ', '.join(nv_proofs))
    if nv_traces:
        print('❌ Non-validated traces for:', ', '.join(nv_traces))

    # Ensure out directory exists
    os.makedirs('out', exist_ok=True)
    out_md = f'out/serializability_report{suffix}.md'
    with open(out_md, 'w') as f:
        f.write(f"# Serializability Analysis Report{' - ' + suffix.title() if suffix else ''}\n"
                f"Generated: {datetime.now():%Y-%m-%d %H:%M:%S}\n"
                f"Extras: {extras}\n\n")
        f.write("|Example|Result|CPU(s)|Valid?|\n|--|--|--|--|\n")
        for r in results:
            # Determine validation status
            if r['original_result'] == 'Serializable':
                valid = 'N/A' if r['proof_verification'] is None else ('✅' if r['proof_verification'] else '❌')
            elif r['original_result'] == 'Not serializable':
                valid = 'N/A' if r['trace_valid'] is None else ('✅' if r['trace_valid'] else '❌')
            else:
                valid = 'N/A'
            # wrap filename in backticks for nicer Markdown
            f.write(f"| `{r['filename']}` |{r['original_result']}|{r['duration']}|{valid}|\n")

        # summary counts
        s_cnt = sum(r['original_result'] == 'Serializable' and r['proof_result'] == 'Serializable'
                    for r in results)
        vp = sum(r['proof_verification'] == True
                 for r in results
                 if r['original_result'] == 'Serializable'
                 and r['proof_result'] == 'Serializable')
        ip = s_cnt - vp
        ns_cnt = sum(r['original_result'] == 'Not serializable' and r['proof_result'] == 'Not serializable'
                     for r in results)
        vt = sum(r['trace_valid'] == True
                 for r in results
                 if r['original_result'] == 'Not serializable'
                 and r['proof_result'] == 'Not serializable')
        it = ns_cnt - vt
        to_cnt = sum(r['status'] == '⏱️ SMPT Timeout' for r in results)
        err = sum(r['status'] == '⚠️ Error' for r in results)
        f.write("\n## Summary\n")
        f.write(f"- Serializable: {s_cnt} (valid proofs: {vp}, invalid: {ip})\n")
        f.write(f"- Not serializable: {ns_cnt} (valid traces: {vt}, invalid: {it})\n")
        f.write(f"- Timeouts: {to_cnt}, Errors: {err}, Total: {len(results)}\n")

    print(f"✅ Done. Report: {out_md}")


def main():
    parser = argparse.ArgumentParser(
        description="Analyze .ser examples with optional cargo flags (optimized only)",
        allow_abbrev=False
    )
    parser.add_argument('--timeout', type=int, help='Timeout seconds')
    parser.add_argument('--jobs', type=int, help='Parallel jobs')
    parser.add_argument('--use-cache', action='store_true', help='Enable caching')
    parser.add_argument('--without-remove-redundant', action='store_true', help='Disable redundant removal')
    parser.add_argument('--without-generate-less', action='store_true', help='Disable generate-less optimization')
    parser.add_argument('--without-smart-kleene-order', action='store_true', help='Disable smart Kleene ordering')
    parser.add_argument('--without-bidirectional', action='store_true',
                        help='Disable bidirectional optimization')  # <— new
    parser.add_argument('--no-viz', action='store_true', help='Disable visualization generation')
    parser.add_argument('--path', type=str, help='Specific file or directory to analyze')
    parser.add_argument('--all-optimizations', action='store_true', 
                        help='Run with all optimizations enabled (default)')
    parser.add_argument('--no-optimizations', action='store_true',
                        help='Run with all optimizations disabled')
    parser.add_argument('--optimization-comparison', action='store_true',
                        help='Run twice: once with all optimizations, once without')
    parser.add_argument('--full-optimization-study', action='store_true',
                        help='Run 6 times: no opts, all opts, and each individual opt')
    known_args, extra = parser.parse_known_args()

    timeout = known_args.timeout
    jobs = known_args.jobs or os.cpu_count() or 4
    cache = known_args.use_cache
    
    # Get files list
    if known_args.path:
        path = Path(known_args.path)
        if path.is_file():
            # Single file
            if path.suffix in ['.ser', '.json']:
                files = [path]
            else:
                parser.error(f"Unsupported file type: {path.suffix}. Only .ser and .json files are supported.")
        elif path.is_dir():
            # Directory - recursively find all .ser and .json files
            files = sorted(
                list(path.rglob('*.ser')) + 
                list(path.rglob('*.json'))
            )
            if not files:
                parser.error(f"No .ser or .json files found in directory: {path}")
        else:
            parser.error(f"Path does not exist: {path}")
    else:
        # Default behavior - analyze examples directory
        files = sorted(
            list(Path('examples/ser').glob('*.ser')) +
            list(Path('examples/json').glob('*.json'))
        )
    
    # Check for conflicting options
    if known_args.all_optimizations and known_args.no_optimizations:
        parser.error("Cannot use --all-optimizations and --no-optimizations together")
    
    # Handle full optimization study mode
    if known_args.full_optimization_study:
        print("🔬 Running full optimization study (6 configurations)...")
        
        # 1. No optimizations
        print("\n📊 Configuration 1/6: All optimizations disabled")
        extras_noopt = ['--without-bidirectional', '--without-remove-redundant', 
                        '--without-generate-less', '--without-smart-kleene-order']
        if known_args.no_viz:
            extras_noopt.append('--no-viz')
        extras_noopt.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_noopt, "_no_optimizations")
        
        # 2. All optimizations
        print("\n📊 Configuration 2/6: All optimizations enabled")
        extras_all = []
        if known_args.no_viz:
            extras_all.append('--no-viz')
        extras_all.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_all, "_all_optimizations")
        
        # 3. Only bidirectional pruning
        print("\n📊 Configuration 3/6: Only bidirectional pruning")
        extras_b = ['--without-remove-redundant', '--without-generate-less', '--without-smart-kleene-order']
        if known_args.no_viz:
            extras_b.append('--no-viz')
        extras_b.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_b, "_only_bidirectional")
        
        # 4. Only remove redundant
        print("\n📊 Configuration 4/6: Only remove redundant")
        extras_r = ['--without-bidirectional', '--without-generate-less', '--without-smart-kleene-order']
        if known_args.no_viz:
            extras_r.append('--no-viz')
        extras_r.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_r, "_only_remove_redundant")
        
        # 5. Only generate less
        print("\n📊 Configuration 5/6: Only generate less")
        extras_g = ['--without-bidirectional', '--without-remove-redundant', '--without-smart-kleene-order']
        if known_args.no_viz:
            extras_g.append('--no-viz')
        extras_g.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_g, "_only_generate_less")
        
        # 6. Only smart Kleene order
        print("\n📊 Configuration 6/6: Only smart Kleene order")
        extras_s = ['--without-bidirectional', '--without-remove-redundant', '--without-generate-less']
        if known_args.no_viz:
            extras_s.append('--no-viz')
        extras_s.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_s, "_only_smart_kleene")
        
        print("\n✅ Full optimization study complete!")
        print("📋 Generated reports:")
        print("  - out/serializability_report_no_optimizations.md")
        print("  - out/serializability_report_all_optimizations.md")
        print("  - out/serializability_report_only_bidirectional.md")
        print("  - out/serializability_report_only_remove_redundant.md")
        print("  - out/serializability_report_only_generate_less.md")
        print("  - out/serializability_report_only_smart_kleene.md")
        return
    
    # Handle optimization comparison mode
    if known_args.optimization_comparison:
        print("🔄 Running optimization comparison...")
        # First run with all optimizations
        print("\n📊 Pass 1: With all optimizations enabled")
        extras_opt = []
        if known_args.no_viz:
            extras_opt.append('--no-viz')
        extras_opt.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_opt, "_optimized")
        
        # Second run without optimizations
        print("\n📊 Pass 2: With all optimizations disabled")
        extras_noopt = ['--without-bidirectional', '--without-remove-redundant', 
                        '--without-generate-less', '--without-smart-kleene-order']
        if known_args.no_viz:
            extras_noopt.append('--no-viz')
        extras_noopt.extend(extra)
        run_analysis(files, timeout, jobs, cache, extras_noopt, "_unoptimized")
        
        print("\n✅ Optimization comparison complete!")
        return
    
    # collect flags to forward
    extras = []
    if known_args.no_optimizations:
        extras.extend(['--without-bidirectional', '--without-remove-redundant', 
                       '--without-generate-less', '--without-smart-kleene-order'])
    else:
        # Individual optimization flags
        if known_args.without_remove_redundant:
            extras.append('--without-remove-redundant')
        if known_args.without_generate_less:
            extras.append('--without-generate-less')
        if known_args.without_smart_kleene_order:
            extras.append('--without-smart-kleene-order')
        if known_args.without_bidirectional:
            extras.append('--without-bidirectional')
    
    # Add --no-viz flag if specified
    if known_args.no_viz:
        extras.append('--no-viz')
    
    # append other unknown flags
    extras.extend(extra)
    
    run_analysis(files, timeout, jobs, cache, extras)


if __name__ == '__main__':
    main()
