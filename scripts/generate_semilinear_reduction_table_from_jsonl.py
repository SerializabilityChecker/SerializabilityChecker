#!/usr/bin/env python3
import os
import sys
import math
import json
import pandas as pd

# ───────────────────────────────────────────────────────────────────────────────
# 1. Paths (hard‐coded)
# ───────────────────────────────────────────────────────────────────────────────
JSONL_FILE = "out/serializability_stats.jsonl"
OUTPUT_TEX = "tex/tables/semilinear_size_reduction.tex"

# ───────────────────────────────────────────────────────────────────────────────
# 2. Load JSONL into DataFrame and expand the 'options' dict
# ───────────────────────────────────────────────────────────────────────────────
with open(JSONL_FILE, "r") as f:
    records = [json.loads(line) for line in f]
df = pd.DataFrame(records)

opts = df["options"].apply(pd.Series)
df = pd.concat([df.drop(columns=["options"]), opts], axis=1)

# ───────────────────────────────────────────────────────────────────────────────
# 2.5. Remove *entire* examples that ever timed out in any scenario
# ───────────────────────────────────────────────────────────────────────────────
# this is because there are instances that we short-circuit T.O. if the semilinear set is too large
# find all examples with at least one timeout
timeout_examples = set(df.loc[df["result"] == "timeout", "example"])
# drop them entirely
df = df[~df["example"].isin(timeout_examples)].copy()

# ───────────────────────────────────────────────────────────────────────────────
# 3. Define the four target optimization scenarios
# ───────────────────────────────────────────────────────────────────────────────
scenarios = {
    "all optimizations (baseline)": {
        "bidirectional_pruning": True,
        "remove_redundant":     True,
        "generate_less":        True,
        "smart_kleene_order":   True,
    },
    "without remove-redundant": {
        "bidirectional_pruning": True,
        "remove_redundant":     False,
        "generate_less":        True,
        "smart_kleene_order":   True,
    },
    "without generate-less": {
        "bidirectional_pruning": True,
        "remove_redundant":     True,
        "generate_less":        False,
        "smart_kleene_order":   True,
    },
    "without smart-kleene-order": {
        "bidirectional_pruning": True,
        "remove_redundant":     True,
        "generate_less":        True,
        "smart_kleene_order":   False,
    },
}

# ───────────────────────────────────────────────────────────────────────────────
# 4. Find examples present under *all* four scenarios
# ───────────────────────────────────────────────────────────────────────────────
sets = []
for combo in scenarios.values():
    mask = pd.Series(True, index=df.index)
    for k, v in combo.items():
        mask &= (df[k] == v)
    sets.append(set(df.loc[mask, "example"].unique()))

common_examples = set.intersection(*sets)
if not common_examples:
    print("Error: no examples appear in all four scenarios → nothing to tabulate.")
    sys.exit(1)

# ───────────────────────────────────────────────────────────────────────────────
# 5. Per‐scenario, keep only the first row per example
# ───────────────────────────────────────────────────────────────────────────────
filtered = {}
for name, combo in scenarios.items():
    mask = pd.Series(True, index=df.index)
    for k, v in combo.items():
        mask &= (df[k] == v)
    sub = df.loc[mask & df["example"].isin(common_examples)].copy()
    sub = sub.sort_values("timestamp").drop_duplicates("example", keep="first")
    filtered[name] = sub

# ───────────────────────────────────────────────────────────────────────────────
# 6. Compute raw stats into a dict
# ───────────────────────────────────────────────────────────────────────────────
stats = {}
for name, sub in filtered.items():
    # num_components
    num_comps = sub["semilinear_set"].apply(lambda s: s["num_components"])
    # avg periods per comp, per example
    avg_periods = sub["semilinear_set"].apply(
        lambda s: (
            sum(c["periods"] for c in s["components"]) / s["num_components"]
        ) if s["num_components"] > 0 else 0
    )
    # max periods per comp, per example
    max_periods = sub["semilinear_set"].apply(
        lambda s: max((c["periods"] for c in s["components"]), default=0)
    )
    stats[name] = {
        "mean_num_components": num_comps.mean(),
        "max_num_components":  num_comps.max(),
        "mean_periods":        avg_periods.mean(),
        "max_periods":         max_periods.max(),
    }

# ───────────────────────────────────────────────────────────────────────────────
# 7. Create stats_df and find column‐wise maxima
# ───────────────────────────────────────────────────────────────────────────────
stats_df = pd.DataFrame.from_dict(stats, orient="index")

max_mean_comps = stats_df["mean_num_components"].max()
max_max_comps  = stats_df["max_num_components"].max()
max_mean_per   = stats_df["mean_periods"].max()
max_max_per    = stats_df["max_periods"].max()

# ───────────────────────────────────────────────────────────────────────────────
# 8. Formatters (and bold the maxima)
# ───────────────────────────────────────────────────────────────────────────────
def fmt_float(x):
    s = f"{x:,.2f}"
    return s.replace(",", "{,}")
    # always round upward, then format as integer
    #return fmt_int(math.ceil(x))

def fmt_int(x):
    s = f"{int(x):,}"
    return s.replace(",", "{,}")

rows = []
for name, row in stats_df.iterrows():
    scen = name.replace("_", "\\_")
    # mean components
    mnc = fmt_float(row["mean_num_components"])
    if row["mean_num_components"] == max_mean_comps:
        mnc = f"\\textbf{{{mnc}}}"
    # max components
    xnc = fmt_int(row["max_num_components"])
    if row["max_num_components"] == max_max_comps:
        xnc = f"\\textbf{{{xnc}}}"
    # mean periods
    mp = fmt_float(row["mean_periods"])
    if row["mean_periods"] == max_mean_per:
        mp = f"\\textbf{{{mp}}}"
    # max periods
    xp = fmt_int(row["max_periods"])
    if row["max_periods"] == max_max_per:
        xp = f"\\textbf{{{xp}}}"
    rows.append((scen, mnc, xnc, mp, xp))

# ───────────────────────────────────────────────────────────────────────────────
# 9. Emit the LaTeX table
# ───────────────────────────────────────────────────────────────────────────────
os.makedirs(os.path.dirname(OUTPUT_TEX), exist_ok=True)
with open(OUTPUT_TEX, "w") as f:
    f.write(r"""\begin{table}[H]
	\centering
	\begin{tabular}{l c c c c}
		\toprule
		& \multicolumn{2}{c}{number of components} & \multicolumn{2}{c}{periods per component} \\
		\cmidrule(lr){2-3} \cmidrule(lr){4-5}
		& average & max & average & max \\
		\midrule
""")
    for scen, mnc, xnc, mp, xp in rows:
        f.write(f"	{scen} & {mnc} & {xnc} & {mp} & {xp} \\\\\n")
    f.write(r"""  \bottomrule
	\end{tabular}
\end{table}
""")

print(f"Wrote LaTeX table to: {OUTPUT_TEX}")
