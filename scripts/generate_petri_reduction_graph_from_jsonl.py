#!/usr/bin/env python3
import json
import os

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

# ───────────────────────────────────────────────────────────────────────────────
# 1. Paths (hard‐coded)
# ───────────────────────────────────────────────────────────────────────────────
JSONL_FILE = "out/serializability_stats.jsonl"
OUTPUT_DIR = "out"
PLOT_PATH  = os.path.join(OUTPUT_DIR, "petri_size_reduction_plot.pdf")

# ───────────────────────────────────────────────────────────────────────────────
# 2. Load & filter
# ───────────────────────────────────────────────────────────────────────────────
with open(JSONL_FILE, 'r') as f:
    records = [json.loads(line) for line in f]

df = pd.DataFrame(records)

# keep only rows where all four options are true
mask = (
    df["options"].apply(lambda o: o.get("bidirectional_pruning", False)) &
    df["options"].apply(lambda o: o.get("remove_redundant",    False)) &
    df["options"].apply(lambda o: o.get("generate_less",       False)) &
    df["options"].apply(lambda o: o.get("smart_kleene_order",  False))
)
df = df[mask]

# ───────────────────────────────────────────────────────────────────────────────
# 3. First‐seen per example
# ───────────────────────────────────────────────────────────────────────────────
df = df.drop_duplicates(subset=["example"], keep="first")

# ───────────────────────────────────────────────────────────────────────────────
# 4. Build a “long” table with stages
# ───────────────────────────────────────────────────────────────────────────────
rows = []
for _, row in df.iterrows():
    places_before      = row["petri_net"]["places_before"]
    trans_before       = row["petri_net"]["transitions_before"]
    disjuncts          = row["petri_net"].get("disjuncts", [])
    # pre‐pruning row
    rows.append({
        "stage":          "pre_pruning",
        "num_places":     places_before,
        "num_transitions":trans_before
    })
    # post‐pruning rows
    if disjuncts:
        for d in disjuncts:
            rows.append({
                "stage":           "post_pruning",
                "num_places":      d["places_after"],
                "num_transitions": d["transitions_after"]
            })
    else:
        # if no disjuncts, treat post = pre
        rows.append({
            "stage":           "post_pruning",
            "num_places":      places_before,
            "num_transitions": trans_before
        })

long_df = pd.DataFrame(rows)

# ───────────────────────────────────────────────────────────────────────────────
# 5. Aggregate means
# ───────────────────────────────────────────────────────────────────────────────
agg = long_df.groupby("stage").agg({
    "num_places":     "mean",
    "num_transitions":"mean"
}).reindex(["pre_pruning", "post_pruning"])

pre_places  = agg.loc["pre_pruning",  "num_places"]
post_places = agg.loc["post_pruning", "num_places"]
pre_trans   = agg.loc["pre_pruning",  "num_transitions"]
post_trans  = agg.loc["post_pruning", "num_transitions"]

# ───────────────────────────────────────────────────────────────────────────────
# 6. Plotting
# ───────────────────────────────────────────────────────────────────────────────
categories = ["Places", "Transitions"]
x = np.arange(len(categories))
width = 0.35

fig, ax = plt.subplots(figsize=(10, 6))

# bars
ax.bar(x[0] - width/2, pre_places,  width,
       label="Before",
       facecolor="forestgreen", edgecolor="black", linewidth=1)
ax.bar(x[0] + width/2, post_places, width,
       label="After",
       facecolor="lightgreen", edgecolor="black", linewidth=1)

ax.bar(x[1] - width/2, pre_trans,   width,
       facecolor="darkorange", edgecolor="black", linewidth=1)
ax.bar(x[1] + width/2, post_trans,  width,
       facecolor="peachpuff", edgecolor="black", linewidth=1)

# annotations
max_h = max(pre_places, post_places, pre_trans, post_trans)
yoff  = max_h * 0.02
fs    = 25

for i, h in enumerate([pre_places, post_places]):
    xpos = x[0] + (i*2-1)*width/2
    ax.text(xpos, h + yoff, ["Before", "After"][i],
            ha="center", fontsize=fs)

for i, h in enumerate([pre_trans, post_trans]):
    xpos = x[1] + (i*2-1)*width/2
    ax.text(xpos, h + yoff, ["Before", "After"][i],
            ha="center", fontsize=fs)

# style
ax.set_xticks(x)
ax.set_xticklabels(categories)
ax.set_ylabel("Average Count", fontsize=fs)
ax.tick_params(axis='x', labelsize=fs)
ax.tick_params(axis='y', labelsize=fs)
ax.set_ylim(0, max_h * 1.1)
ax.yaxis.grid(True, linestyle="--", linewidth=0.5, alpha=0.7)

fig.tight_layout()

# ensure output dir exists
os.makedirs(OUTPUT_DIR, exist_ok=True)
fig.savefig(PLOT_PATH)
print(f"Plot saved to: {PLOT_PATH}")
