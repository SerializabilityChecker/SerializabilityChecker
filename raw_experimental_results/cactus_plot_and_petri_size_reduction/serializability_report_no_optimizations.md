# Serializability Analysis Report - _No_Optimizations
Generated: 2025-07-03 19:47:05
Extras: ['--without-bidirectional', '--without-remove-redundant', '--without-generate-less', '--without-smart-kleene-order']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|1.04|✅|
| `b2` |SMPT Timeout|0.27|N/A|
| `b3` |SMPT Timeout|0.15|N/A|
| `b4` |SMPT Timeout|0.16|N/A|
| `a1` |Serializable|0.17|✅|
| `a2` |Not serializable|0.46|✅|
| `a3` |Serializable|0.16|✅|
| `a4` |Serializable|1.40|✅|
| `a5` |SMPT Timeout|0.19|N/A|
| `a6` |Not serializable|0.74|✅|
| `a7` |Serializable|0.18|✅|
| `c1` |SMPT Timeout|0.00|N/A|
| `c2` |SMPT Timeout|0.00|N/A|
| `c3` |Serializable|2.58|✅|
| `c4` |SMPT Timeout|1.72|N/A|
| `c5` |Not serializable|0.69|✅|
| `c6` |Not serializable|0.56|✅|
| `c7` |Not serializable|0.61|✅|
| `c8` |SMPT Timeout|1.57|N/A|
| `d1` |Serializable|3.85|✅|
| `d2` |Not serializable|0.52|✅|
| `d3` |Serializable|5.37|✅|
| `d4` |Serializable|8.70|✅|
| `d5` |Not serializable|0.43|✅|
| `e1` |Serializable|0.60|✅|
| `e2` |SMPT Timeout|0.79|N/A|
| `e3` |Not serializable|3.44|✅|
| `e4` |SMPT Timeout|4.10|N/A|
| `e5` |Serializable|0.15|✅|
| `e6` |Serializable|0.19|✅|
| `e7` |Serializable|0.47|✅|
| `f1` |Serializable|10.70|✅|
| `f2` |Not serializable|0.30|✅|
| `f3` |Not serializable|0.32|✅|
| `f4` |Serializable|8.29|✅|
| `f5` |SMPT Timeout|0.00|N/A|
| `f6` |Not serializable|118.08|✅|
| `f7` |Not serializable|0.26|✅|
| `f8` |SMPT Timeout|0.28|N/A|
| `f9` |Serializable|0.11|✅|
| `g1` |SMPT Timeout|14.22|N/A|
| `g2` |SMPT Timeout|0.00|N/A|
| `g3` |SMPT Timeout|0.00|N/A|
| `g4` |Not serializable|0.93|✅|
| `g5` |Serializable|41.40|✅|
| `g6` |Not serializable|4.76|✅|
| `g7` |Serializable|11.20|✅|

## Summary
- Serializable: 18 (valid proofs: 18, invalid: 0)
- Not serializable: 14 (valid traces: 14, invalid: 0)
- Timeouts: 15, Errors: 0, Total: 47
