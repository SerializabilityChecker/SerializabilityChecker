#!/usr/bin/env python3
import json
import csv
import os

# Hard-coded paths
INPUT_JSONL = "out/serializability_stats.jsonl"
SUMMARY_CSV = "out/jsonl_summarizing_table.csv"
OUTPUT_TEX = "out/tables/big_table_summary.tex"

# timeout threshold
TIMEOUT_IN_SECOND = 500
TIMEOUT_MS = 1000 * TIMEOUT_IN_SECOND

# map CSV result → LaTeX symbol
SYMBOLS = {
    "serializable": r"\greencmark",
    "not_serializable": r"\xmark",
    "timeout": r"\textbf{?}"
}

# when a timeout occurs, override based on these sets
HARD_SERIALIZABLE = {"g2.ser"}
HARD_NON_SERIALIZABLE = {"c1.ser", "e2.ser"}

# categories and their benchmarks (in order)
CATEGORIES = [
    ("Core expressions", ["a1.ser","a2.ser","a3.ser","a4.ser","a5.ser","a6.ser","a7.ser"]),
    ("State machines", ["b1.json","b2.json","b3.json","b4.json"]),
    ("Mixed arithmetic", ["c1.ser","c2.ser","c3.ser","c4.ser","c5.ser","c6.ser","c7.ser","c8.ser"]),
    ("Circular increment", ["d1.ser","d2.ser","d3.ser","d4.ser","d5.ser"]),
    ("Concurrency \\& locking loops", ["e1.ser","e2.ser","e3.ser","e4.ser","e5.ser","e6.ser","e7.ser"]),
    ("Non-deterministic \\& randomness", ["f1.ser","f2.ser","f3.ser","f4.ser","f5.ser","f6.ser","f7.ser","f8.ser","f9.ser"]),
    ("Networking \\& system protocols", ["g1.ser","g2.ser","g3.ser","g4.ser","g5.ser","g6.ser","g7.ser"])
]

# static features columns (six entries) for each benchmark
FEATURES_COLS = {
    "a1.ser":   "&  & \\cmark &  &  &       &  ",
    "a2.ser":   "&  &        &  &  & \\cmark &  ",
    "a3.ser":   "&  &        &  &  &       &  ",
    "a4.ser":   "&  &        &  &  & \\cmark & \\cmark",
    "a5.ser":   "&  & \\cmark &  &  & \\cmark & \\cmark",
    "a6.ser":   "&  &        &  &  & \\cmark & \\cmark",
    "a7.ser":   "& \\cmark & \\cmark &  &  & \\cmark &  ",
    "b1.json":  "& \\cmark &        &  &  & \\cmark & \\cmark",
    "b2.json":  "& \\cmark &        &  &  & \\cmark & \\cmark",
    "b3.json":  "& \\cmark &        &  &  & \\cmark & \\cmark",
    "b4.json":  "& \\cmark &        &  &  & \\cmark & \\cmark",
    "c1.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "c2.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "c3.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "c4.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "c5.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "c6.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "c7.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "c8.ser":   "&  & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "d1.ser":   "& \\cmark & \\cmark & \\cmark &  & \\cmark &  ",
    "d2.ser":   "& \\cmark &        & \\cmark &  &   \\cmark &  ",
    "d3.ser":   "& \\cmark & \\cmark & \\cmark &  &  \\cmark &  ",
    "d4.ser":   "& \\cmark & \\cmark & \\cmark &  &     \\cmark &  ",
    "d5.ser":   "& \\cmark &        &  &  & \\cmark &  ",
    "e1.ser":   "&  & \\cmark &  &  & \\cmark &  ",
    "e2.ser":   "& \\cmark & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "e3.ser":   "& \\cmark & \\cmark &  & \\cmark &   \\cmark & \\cmark",
    "e4.ser":   "& \\cmark & \\cmark &  &  \\cmark &   \\cmark & \\cmark",
    "e5.ser":   "& \\cmark & \\cmark & \\cmark &  & \\cmark &  ",
    "e6.ser":   "& \\cmark & \\cmark & \\cmark &  & \\cmark &  ",
    "e7.ser":   "&  & \\cmark &  &  &   \\cmark &  ",
    "f1.ser":   "& \\cmark &    \\cmark    & \\cmark &  & \\cmark &  ",
    "f2.ser":   "& \\cmark &   \\cmark     & \\cmark &  & \\cmark &  ",
    "f3.ser":   "&  &        &  & \\cmark &   \\cmark & \\cmark",
    "f4.ser":   "&  &     \\cmark   &  & \\cmark & \\cmark & \\cmark",
    "f5.ser":   "& \\cmark &        & \\cmark &  &       &  ",
    "f6.ser":   "& \\cmark &        & \\cmark &  & \\cmark &  ",
    "f7.ser":   "& \\cmark &        & \\cmark &  &  \\cmark &  ",
    "f8.ser":   "& \\cmark &        & \\cmark &  &   \\cmark &  ",
    "f9.ser":   "& \\cmark &        & \\cmark &  &  \\cmark &  ",
    "g1.ser":   "& \\cmark & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "g2.ser":   "& \\cmark & \\cmark &  & \\cmark & \\cmark & \\cmark",
    "g3.ser":   "& \\cmark & \\cmark & \\cmark & \\cmark & \\cmark & \\cmark",
    "g4.ser":   "& \\cmark & \\cmark & \\cmark & \\cmark & \\cmark & \\cmark",
    "g5.ser":   "& \\cmark & \\cmark & \\cmark & \\cmark &   \\cmark & \\cmark",
    "g6.ser":   "& \\cmark &        & \\cmark & \\cmark & \\cmark &  ",
    "g7.ser":   "& \\cmark &        & \\cmark & \\cmark &       &  ",
}

def summarize_jsonl_to_csv(input_path, output_path):
    """Read JSONL and write benchmark summary CSV."""
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(input_path) as jf, open(output_path, "w", newline="") as cf:
        writer = csv.writer(cf)
        writer.writerow([
            "benchmark",
            "result",
            "certificate running time",
            "total running time"
        ])
        for line in jf:
            if not line.strip():
                continue
            rec = json.loads(line)
            bm = os.path.basename(rec.get("example", ""))
            res = rec.get("result", "")
            cert = rec.get("certificate_creation_time_ms", "")
            total = rec.get("total_time_ms", "")
            writer.writerow([bm, res, cert, total])
    print(f"Wrote CSV summary to {output_path}")

def load_summary(csv_path):
    """Load CSV into dict bench→(res, cert, total)."""
    summary = {}
    with open(csv_path) as cf:
        reader = csv.DictReader(cf)
        for row in reader:
            summary[row["benchmark"]] = (
                row["result"],
                row["certificate running time"],
                row["total running time"]
            )
    return summary

def generate_table(summary, out_path):
    """Write the LaTeX table using summary data and timeout logic."""
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as tex:
        # header
        tex.write(r"""\begin{table}[H]
	\centering
	\small
	% increase horizontal padding between columns
	\setlength{\tabcolsep}{5pt}
	\renewcommand{\arraystretch}{0.9}
	\begin{tabular*}{\textwidth}{@{\extracolsep{\fill}}%
			p{1.5cm}   % Category
			p{1.0cm} % Benchmark
			c        % Serializable
			c c c c c c % Features
			r r       % Cert, Total
		}
		\toprule
		\multicolumn{2}{c}{\textbf{Benchmark}}
		& \textbf{Serializable}
		& \multicolumn{6}{c}{\textbf{Features}}
		& \multicolumn{2}{c}{\textbf{Runtime (ms)}} \\
		\cmidrule(lr){1-2} \cmidrule(lr){3-3} \cmidrule(lr){4-9} \cmidrule(lr){10-11}
		&
		&
		& If & While & \texttt{?} & Arith & Yield & Multi-req
		& Cert. & Total \\
		\midrule
""")
        # body
        for cat, benches in CATEGORIES:
            tex.write(f"\t\t\\multirow{{{len(benches)}}}{{=}}{{{cat}}}")
            for i, bm in enumerate(benches):
                res, cert, total = summary.get(bm, ("", "", ""))
                # pick symbol, with override on timeout
                if res == "timeout":
                    if bm in HARD_SERIALIZABLE:
                        sym = SYMBOLS["serializable"]
                    elif bm in HARD_NON_SERIALIZABLE:
                        sym = SYMBOLS["not_serializable"]
                    else:
                        sym = SYMBOLS["timeout"]
                else:
                    sym = SYMBOLS.get(res, "")

                # timing display
                if res == "timeout":
                    cert_disp = r"\texttt{TIMEOUT}"
                    total_disp = r"\texttt{TIMEOUT}"
                else:
                    cert_disp = cert
                    try:
                        total_ms = int(total)
                    except ValueError:
                        total_ms = 0
                    if total_ms >= TIMEOUT_MS:
                        total_disp = r"\texttt{TIMEOUT}"
                    else:
                        total_disp = total

                feats = FEATURES_COLS[bm]
                prefix = "" if i == 0 else "\t\t"
                tex.write(f"{prefix} & \\texttt{{{bm}}} & {sym} {feats} & {cert_disp} & {total_disp} \\\\\n")
            tex.write("\t\t\\midrule\n")
        # footer
        tex.write(r"""\bottomrule
	\end{tabular*}
\end{table}
""")
    print(f"Wrote LaTeX table to {out_path}")

def main():
    summarize_jsonl_to_csv(INPUT_JSONL, SUMMARY_CSV)
    summary = load_summary(SUMMARY_CSV)
    generate_table(summary, OUTPUT_TEX)

if __name__ == "__main__":
    main()
