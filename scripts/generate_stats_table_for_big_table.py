#!/usr/bin/env python3
import pandas as pd

INPUT_CSV_PATH = "out/jsonl_summarizing_table.csv"
OUT_TEX_TABLE_PATH = "tex/tables/average_and_mean_values_of_big_table.tex"

def format_int(value: float) -> str:
    """Round a float to the nearest integer and return as string."""
    return str(int(round(value)))

def compute_stats(df: pd.DataFrame, col: str):
    """Return average and median (rounded to int) for a column."""
    return format_int(df[col].mean()), format_int(df[col].median())

def main():
    # Read input CSV
    df = pd.read_csv(INPUT_CSV_PATH)

    # Column names
    cert_col   = "certificate running time"
    total_col  = "total running time"
    result_col = "result"
    val_col    = "certificate validation"

    # Compute validation time column
    df[val_col] = df[total_col] - df[cert_col]

    # Masks for categories
    ser     = df[result_col] == "serializable"
    non_ser = df[result_col] == "not_serializable"

    # Compute stats for each group
    stats = {
        "Serializable": {
            "cert_avg": compute_stats(df[ser],     cert_col)[0],
            "cert_med": compute_stats(df[ser],     cert_col)[1],
            "val_avg":  compute_stats(df[ser],     val_col)[0],
            "val_med":  compute_stats(df[ser],     val_col)[1],
            "tot_avg":  compute_stats(df[ser],     total_col)[0],
            "tot_med":  compute_stats(df[ser],     total_col)[1],
        },
        "Not serializable": {
            "cert_avg": compute_stats(df[non_ser], cert_col)[0],
            "cert_med": compute_stats(df[non_ser], cert_col)[1],
            "val_avg":  compute_stats(df[non_ser], val_col)[0],
            "val_med":  compute_stats(df[non_ser], val_col)[1],
            "tot_avg":  compute_stats(df[non_ser], total_col)[0],
            "tot_med":  compute_stats(df[non_ser], total_col)[1],
        },
        "All": {
            "cert_avg": compute_stats(df,           cert_col)[0],
            "cert_med": compute_stats(df,           cert_col)[1],
            "val_avg":  compute_stats(df,           val_col)[0],
            "val_med":  compute_stats(df,           val_col)[1],
            "tot_avg":  compute_stats(df,           total_col)[0],
            "tot_med":  compute_stats(df,           total_col)[1],
        },
    }

    # Build LaTeX table
    lines = []
    lines.append(r"\begin{table}[H]")
    lines.append("\t\\centering")
    lines.append("\t\\begin{tabular}{lrrrrrr}")
    lines.append("\t\t\\toprule")
    lines.append("\t\t& \\multicolumn{3}{c}{Average time (ms)} & \\multicolumn{3}{c}{Median time (ms)} \\\\")
    lines.append("\t\t\\cmidrule(lr){2-4} \\cmidrule(lr){5-7}")
    lines.append("\t\tCategory")
    lines.append("\t\t& \\shortstack{certificate\\\\generation}")
    lines.append("\t\t& \\shortstack{certificate\\\\validation}")
    lines.append("\t\t& total")
    lines.append("\t\t& \\shortstack{certificate\\\\generation}")
    lines.append("\t\t& \\shortstack{certificate\\\\validation}")
    lines.append("\t\t& total \\\\")
    lines.append("\t\t\\midrule")

    for cat, vals in stats.items():
        lines.append(
            f"\t\t{cat:<17} & "
            f"{vals['cert_avg']:>6} & "
            f"{vals['val_avg']:>6} & "
            f"{vals['tot_avg']:>6} & "
            f"{vals['cert_med']:>5} & "
            f"{vals['val_med']:>5} & "
            f"{vals['tot_med']:>5} \\\\"
        )

    lines.append("\t\t\\bottomrule")
    lines.append("\t\\end{tabular}")
    lines.append(r"\end{table}")

    # Write to file
    with open(OUT_TEX_TABLE_PATH, 'w') as f:
        f.write("\n".join(lines))

if __name__ == "__main__":
    main()
