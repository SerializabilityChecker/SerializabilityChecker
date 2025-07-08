#!/usr/bin/env python3
"""
Generate LaTeX tables from serializability statistics
"""

import json
from pathlib import Path
import argparse
from collections import defaultdict
from typing import List, Dict, Any

# Try to import plotting libraries
try:
    import matplotlib.pyplot as plt
    import numpy as np
    PLOTTING_AVAILABLE = True
except ImportError:
    PLOTTING_AVAILABLE = False
    print("Warning: matplotlib not available. Cactus plot generation will be skipped.")


def load_stats(stats_file) -> List[Dict[str, Any]]:
    """Load all statistics from JSONL file"""
    stats = []
    with open(stats_file, 'r') as f:
        for line in f:
            if line.strip():
                stats.append(json.loads(line.strip()))
    return stats


def format_time(ms):
    """Format milliseconds to a nice string"""
    if ms is None:
        return "N/A"
    if ms < 1000:
        return f"{ms:.0f}ms"
    else:
        return f"{ms/1000:.1f}s"


def generate_comprehensive_table(stats: List[Dict[str, Any]]) -> str:
    """Generate comprehensive statistics table"""
    # Group by example and optimization settings
    grouped = defaultdict(list)
    for stat in stats:
        key = (stat['example'],
               stat['options']['bidirectional_pruning'],
               stat['options']['remove_redundant'],
               stat['options']['generate_less'],
               stat['options']['smart_kleene_order'])
        grouped[key].append(stat)

    rows = []
    for (example, bid, rem, gen, smart), group in sorted(grouped.items()):
        row = group[0]  # Assuming one run per configuration

        # Extract base name without extension
        example_name = Path(example).stem

        # Format optimization flags as a compact string
        opts = f"{'B' if bid else '-'}{'R' if rem else '-'}{'G' if gen else '-'}{'S' if smart else '-'}"

        # Calculate pruning effectiveness
        total_places_removed = sum(d['removed_places'] for d in row['petri_net']['disjuncts'])
        total_trans_removed = sum(d['removed_transitions'] for d in row['petri_net']['disjuncts'])

        places_before = row['petri_net']['places_before']
        trans_before = row['petri_net']['transitions_before']

        pruning_pct = 0
        if trans_before > 0:
            pruning_pct = (total_trans_removed / trans_before) * 100

        rows.append({
            'Example': f"\\texttt{{{example_name}}}",
            'Opts': opts,
            'Result': 'S' if row['result'] == 'serializable' else ('T' if row['result'] == 'timeout' else 'NS'),
            'Disjuncts': str(row['num_disjuncts']),
            'SL Size': str(row['semilinear_set']['num_components']),
            'PN Size': f"{places_before}/{trans_before}",
            'Pruned': f"{pruning_pct:.0f}\\%",
            'Time': format_time(row['total_time_ms']),
            'SMPT': f"{row['smpt_calls']}/{row['smpt_timeouts']}"
        })

    # Convert to LaTeX
    latex = "\\begin{tabular}{llccccccc}\n"
    latex += "\\toprule\n"
    latex += "Example & Opts & Result & Disj. & SL & PN & Pruned & Time & SMPT \\\\\n"
    latex += "\\midrule\n"

    for row in rows:
        latex += " & ".join([row['Example'], row['Opts'], row['Result'], row['Disjuncts'],
                           row['SL Size'], row['PN Size'], row['Pruned'], row['Time'], row['SMPT']]) + " \\\\\n"

    latex += "\\bottomrule\n"
    latex += "\\end{tabular}\n"
    latex += "\n% Legend: S = Serializable, NS = Not Serializable, T = Timeout\n"
    latex += "% Opts: B = Bidirectional Pruning, R = Remove Redundant, G = Generate Less, S = Smart Kleene Order\n"

    return latex


def generate_pruning_effectiveness_table(stats: List[Dict[str, Any]]) -> str:
    """Generate table showing Petri net size reduction"""
    # Filter for runs with all optimizations enabled
    opt_stats = [s for s in stats if (s['options']['bidirectional_pruning'] and
                                      s['options']['remove_redundant'] and
                                      s['options']['generate_less'] and
                                      s['options']['smart_kleene_order'])]

    rows = []
    for stat in opt_stats:
        example_name = Path(stat['example']).stem

        for d in stat['petri_net']['disjuncts']:
            reduction_pct = 0
            if stat['petri_net']['transitions_before'] > 0:
                reduction_pct = (d['removed_transitions'] / stat['petri_net']['transitions_before']) * 100

            rows.append({
                'Example': f"\\texttt{{{example_name}}}",
                'Disjunct': str(d['id']),
                'Places': f"{stat['petri_net']['places_before']} → {d['places_after']}",
                'Transitions': f"{stat['petri_net']['transitions_before']} → {d['transitions_after']}",
                'Iterations': str(d['pruning_iterations']),
                'Reduction': f"{reduction_pct:.1f}\\%"
            })

    # Convert to LaTeX
    latex = "\\begin{tabular}{llcccc}\n"
    latex += "\\toprule\n"
    latex += "Example & Disj. & Places & Transitions & Iter. & Reduction \\\\\n"
    latex += "\\midrule\n"

    for row in rows:
        latex += " & ".join([row['Example'], row['Disjunct'], row['Places'],
                           row['Transitions'], row['Iterations'], row['Reduction']]) + " \\\\\n"

    latex += "\\bottomrule\n"
    latex += "\\end{tabular}\n"

    return latex


def generate_timing_comparison_table(stats: List[Dict[str, Any]]) -> str:
    """Generate table comparing times with/without optimizations"""
    # Group by example
    by_example = defaultdict(dict)

    for stat in stats:
        example_name = Path(stat['example']).stem
        opts_key = 'optimized' if (stat['options']['bidirectional_pruning'] and
                                  stat['options']['remove_redundant'] and
                                  stat['options']['generate_less'] and
                                  stat['options']['smart_kleene_order']) else 'unoptimized'

        by_example[example_name][opts_key] = stat

    rows = []
    for example, data in sorted(by_example.items()):
        if 'optimized' in data and 'unoptimized' in data:
            opt = data['optimized']
            unopt = data['unoptimized']

            speedup = unopt['total_time_ms'] / opt['total_time_ms'] if opt['total_time_ms'] > 0 else 0

            rows.append({
                'Example': f"\\texttt{{{example}}}",
                'Unoptimized': format_time(unopt['total_time_ms']),
                'Optimized': format_time(opt['total_time_ms']),
                'Speedup': f"{speedup:.1f}×",
                'Cert. Create': format_time(opt.get('certificate_creation_time_ms', 0)),
                'Cert. Check': format_time(opt.get('certificate_checking_time_ms', 0))
            })

    # Convert to LaTeX
    latex = "\\begin{tabular}{lccccc}\n"
    latex += "\\toprule\n"
    latex += "Example & Unopt. & Opt. & Speedup & Create & Check \\\\\n"
    latex += "\\midrule\n"

    for row in rows:
        latex += " & ".join([row['Example'], row['Unoptimized'], row['Optimized'],
                           row['Speedup'], row['Cert. Create'], row['Cert. Check']]) + " \\\\\n"

    latex += "\\bottomrule\n"
    latex += "\\end{tabular}\n"

    return latex


def generate_optimization_breakdown_table(stats: List[Dict[str, Any]]) -> str:
    """Generate table showing impact of individual optimizations"""
    # Group by example and optimization configuration
    by_example = defaultdict(dict)

    for stat in stats:
        example_name = Path(stat['example']).stem

        # Determine optimization configuration key
        bid = stat['options']['bidirectional_pruning']
        rem = stat['options']['remove_redundant']
        gen = stat['options']['generate_less']
        smart = stat['options']['smart_kleene_order']

        if not any([bid, rem, gen, smart]):
            opts_key = 'none'
        elif all([bid, rem, gen, smart]):
            opts_key = 'all'
        elif bid and not any([rem, gen, smart]):
            opts_key = 'bidirectional'
        elif rem and not any([bid, gen, smart]):
            opts_key = 'remove_redundant'
        elif gen and not any([bid, rem, smart]):
            opts_key = 'generate_less'
        elif smart and not any([bid, rem, gen]):
            opts_key = 'smart_kleene'
        else:
            continue  # Skip mixed configurations

        by_example[example_name][opts_key] = stat

    rows = []
    for example, data in sorted(by_example.items()):
        if 'none' in data and 'all' in data:
            row = {
                'Example': f"\\texttt{{{example}}}",
                'None': format_time(data['none']['total_time_ms']),
                'All': format_time(data['all']['total_time_ms']),
            }

            # Add individual optimization times
            for opt_name, opt_key in [('B', 'bidirectional'), ('R', 'remove_redundant'),
                                     ('G', 'generate_less'), ('S', 'smart_kleene')]:
                if opt_key in data:
                    row[opt_name] = format_time(data[opt_key]['total_time_ms'])
                else:
                    row[opt_name] = '--'

            rows.append(row)

    if not rows:
        return "% No optimization breakdown data available\n"

    # Convert to LaTeX
    latex = "\\begin{tabular}{lcccccc}\n"
    latex += "\\toprule\n"
    latex += "Example & No Opt & B & R & G & S & All Opt \\\\\n"
    latex += "\\midrule\n"

    for row in rows:
        latex += " & ".join([row['Example'], row['None'], row['B'], row['R'],
                           row['G'], row['S'], row['All']]) + " \\\\\n"

    latex += "\\bottomrule\n"
    latex += "\\end{tabular}\n"
    latex += "\n% B = Bidirectional Pruning, R = Remove Redundant, G = Generate Less, S = Smart Kleene Order\n"

    return latex


def generate_cactus_plot(stats: List[Dict[str, Any]], output_dir: str) -> None:
    """Generate cactus plot (CDF style) showing performance of different optimization configurations"""
    if not PLOTTING_AVAILABLE:
        print("Skipping cactus plot generation (matplotlib not available)")
        return

    # First, get all unique example names
    all_examples = set()
    for stat in stats:
        example_name = Path(stat['example']).stem
        all_examples.add(example_name)

    total_examples = len(all_examples)
    print(f"Total unique examples: {total_examples}")

    # Group by optimization configuration and example
    config_example_times = defaultdict(lambda: defaultdict(list))
    config_example_timeouts = defaultdict(lambda: defaultdict(list))

    for stat in stats:
        # Extract optimization configuration
        bid = stat['options']['bidirectional_pruning']
        rem = stat['options']['remove_redundant']
        gen = stat['options']['generate_less']
        smart = stat['options']['smart_kleene_order']

        # Create config label
        config_label = f"{'B' if bid else '-'}{'R' if rem else '-'}{'G' if gen else '-'}{'S' if smart else '-'}"

        # Get example name
        example_name = Path(stat['example']).stem

        # Get time in seconds
        time_s = stat['total_time_ms'] / 1000.0

        timeout_in_sec = stat['options']['timeout']
        timeout_in_ms = timeout_in_sec * 1000

        # Separate timeouts from successful runs
        if stat['result'] == 'timeout' or stat['total_time_ms']>=timeout_in_ms:
            config_example_timeouts[config_label][example_name].append(time_s)
        else:
            config_example_times[config_label][example_name].append(time_s)

    # Average times for each example within each configuration
    config_avg_times = defaultdict(dict)
    config_timeouts = defaultdict(set)

    # Process successful runs
    for config, example_times in config_example_times.items():
        for example, times in example_times.items():
            # Average multiple runs of the same example
            avg_time = sum(times) / len(times)
            config_avg_times[config][example] = avg_time

    # Process timeouts
    for config, example_timeouts in config_example_timeouts.items():
        for example, times in example_timeouts.items():
            # Mark this example as timed out for this config
            config_timeouts[config].add(example)

    # Set up the plot
    try:
        plt.style.use('seaborn-v0_8-whitegrid')
    except:
        try:
            plt.style.use('seaborn-whitegrid')
        except:
            # Use default style with manual grid
            pass
    fig, ax = plt.subplots(figsize=(10, 6))

    # Define colors for different configurations
    all_configs = set(list(config_avg_times.keys()) + list(config_timeouts.keys()))
    colors = plt.cm.tab10(np.linspace(0, 1, len(all_configs)))

    # Calculate max time across all configurations to set x-axis limit
    max_time = 0
    for config_times in config_avg_times.values():
        if config_times:
            config_max = max(config_times.values())
            max_time = max(max_time, config_max)

    # Add some padding to the max time (10% or at least 1 second)
    if max_time > 0:
        x_max = max_time * 1.1
    else:
        x_max = 10  # Default if no data

    # Plot each configuration
    config_labels = sorted(set(list(config_avg_times.keys()) + list(config_timeouts.keys())))
    for idx, config in enumerate(config_labels):
        example_times = config_avg_times.get(config, {})
        timeouts = config_timeouts.get(config, set())

        # Get only the examples that were actually solved (not timed out)
        times_list = list(example_times.values())

        if times_list:
            # Sort times
            sorted_times = sorted(times_list)

            # Calculate percentage of total examples solved
            # Each solved example contributes 1/total_examples to the percentage
            percentages = [(i + 1) / total_examples * 100 for i in range(len(sorted_times))]

            # Add starting point at 0% from the beginning of the plot
            # This creates the step function effect
            # Also extend to x_max to show the horizontal line
            sorted_times_extended = [0] + sorted_times + [x_max]
            percentages_extended = [0] + percentages + [percentages[-1]]

            # Plot the step function with label showing solved/total for this config
            # total_attempted = len(times_list) + len(timeouts)
            total_attempted = 47
            label = f"{config} ({len(times_list)}/{total_attempted})"
            ax.step(sorted_times_extended, percentages_extended, where='post', linewidth=2.5, label=label,
                    color=colors[idx % len(colors)], alpha=0.8)
        else:
            # No solved examples, just show in legend
            total_attempted = len(timeouts)
            label = f"{config} (0/{total_attempted})"
            # Plot a dummy point outside visible range just to get in legend
            ax.plot([100], [0], linewidth=2.5, label=label,
                    color=colors[idx % len(colors)], marker='o', markersize=4, alpha=0.8)

    # Set up axes
    ax.set_xlabel('Time (seconds)', fontsize=25)
    ax.set_ylabel('Examples Solved (%)', fontsize=25)
    # ax.set_title('Cactus Plot: Optimization Configuration Performance', fontsize=16, pad=20)

    # Set x-axis to linear scale with dynamic upper limit
    ax.set_xlim(0, x_max)
    ax.set_ylim(0, 105)


    plt.xticks(fontsize=25)
    plt.yticks(fontsize=25)

    # Add grid
    ax.grid(True, which='both', alpha=0.3)

    # Add legend
    ax.legend(loc='lower right', title='Configuration', fontsize=20, title_fontsize=20)

    # Add a note about the configuration labels
    # note_text = 'B=Bidirectional, R=Remove Redundant, G=Generate Less, S=Smart Kleene'
    # fig.text(0.5, 0.02, note_text, ha='center', fontsize=10, style='italic')

    # Tight layout with space for note
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.1)

    # Create figures directory if it doesn't exist
    figures_dir = Path(output_dir).parent / 'figures'
    figures_dir.mkdir(parents=True, exist_ok=True)

    # Save in multiple formats
    for fmt in ['pdf', 'png']:
        output_path = figures_dir / f'cactus_plot.{fmt}'
        plt.savefig(output_path, dpi=300, bbox_inches='tight')
        print(f"Saved cactus plot to {output_path}")

    plt.close()


def generate_summary_statistics(stats: List[Dict[str, Any]]) -> str:
    """Generate summary statistics"""
    opt_stats = [s for s in stats if (s['options']['bidirectional_pruning'] and
                                      s['options']['remove_redundant'] and
                                      s['options']['generate_less'] and
                                      s['options']['smart_kleene_order'])]

    if not opt_stats:
        return "\\textit{No statistics available with all optimizations enabled.}\n"

    # Calculate various statistics
    total_examples = len(opt_stats)
    serializable = len([s for s in opt_stats if s['result'] == 'serializable'])
    not_serializable = len([s for s in opt_stats if s['result'] == 'not_serializable'])
    timeouts = len([s for s in opt_stats if s['result'] == 'timeout'])

    # Average pruning effectiveness
    pruning_percentages = []
    for stat in opt_stats:
        trans_before = stat['petri_net']['transitions_before']
        if trans_before > 0:
            total_removed = sum(d['removed_transitions'] for d in stat['petri_net']['disjuncts'])
            pruning_percentages.append((total_removed / trans_before) * 100)

    avg_pruning = sum(pruning_percentages) / len(pruning_percentages) if pruning_percentages else 0
    avg_time = sum(s['total_time_ms'] for s in opt_stats) / len(opt_stats) if opt_stats else 0

    summary = f"""\\begin{{itemize}}
\\item Total examples analyzed: {total_examples}
\\item Serializable: {serializable} ({serializable/total_examples*100:.1f}\\%)
\\item Not serializable: {not_serializable} ({not_serializable/total_examples*100:.1f}\\%)
\\item Timeouts: {timeouts} ({timeouts/total_examples*100:.1f}\\%)
\\item Average pruning effectiveness: {avg_pruning:.1f}\\%
\\item Average analysis time: {format_time(avg_time)}
\\end{{itemize}}
"""
    return summary


def main():
    parser = argparse.ArgumentParser(description="Generate LaTeX tables from serializability statistics")
    parser.add_argument('--input', default='out/serializability_stats.jsonl', help='Input JSONL file')
    parser.add_argument('--output-dir', default='tex/tables/', help='Output directory for LaTeX tables')
    parser.add_argument('--skip-plot', action='store_true', help='Skip cactus plot generation')
    args = parser.parse_args()

    # Load data
    print(f"Loading statistics from {args.input}...")
    stats = load_stats(args.input)
    print(f"Loaded {len(stats)} records")

    if not stats:
        print("No statistics found!")
        return

    # Create output directory
    Path(args.output_dir).mkdir(parents=True, exist_ok=True)

    # Generate tables
    print("Generating comprehensive table...")
    with open(f"{args.output_dir}/comprehensive_stats.tex", 'w') as f:
        f.write(generate_comprehensive_table(stats))

    print("Generating pruning effectiveness table...")
    with open(f"{args.output_dir}/pruning_effectiveness.tex", 'w') as f:
        f.write(generate_pruning_effectiveness_table(stats))

    print("Generating timing comparison table...")
    with open(f"{args.output_dir}/timing_comparison.tex", 'w') as f:
        f.write(generate_timing_comparison_table(stats))

    print("Generating optimization breakdown table...")
    with open(f"{args.output_dir}/optimization_breakdown.tex", 'w') as f:
        f.write(generate_optimization_breakdown_table(stats))

    print("Generating summary statistics...")
    with open(f"{args.output_dir}/summary_stats.tex", 'w') as f:
        f.write(generate_summary_statistics(stats))

    if not args.skip_plot:
        print("Generating cactus plot...")
        generate_cactus_plot(stats, args.output_dir)

    print(f"Tables saved to {args.output_dir}")


if __name__ == '__main__':
    main()