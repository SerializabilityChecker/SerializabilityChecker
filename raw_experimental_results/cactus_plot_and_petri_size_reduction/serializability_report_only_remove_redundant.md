# Serializability Analysis Report - _Only_Remove_Redundant
Generated: 2025-07-03 20:03:21
Extras: ['--without-bidirectional', '--without-generate-less', '--without-smart-kleene-order']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.82|✅|
| `b2` |SMPT Timeout|0.18|N/A|
| `b3` |SMPT Timeout|0.09|N/A|
| `b4` |SMPT Timeout|0.10|N/A|
| `a1` |Serializable|0.08|✅|
| `a2` |Not serializable|0.28|✅|
| `a3` |Serializable|0.08|✅|
| `a4` |Serializable|1.22|✅|
| `a5` |SMPT Timeout|0.09|N/A|
| `a6` |Not serializable|0.50|✅|
| `a7` |Serializable|0.10|✅|
| `c1` |SMPT Timeout|0.00|N/A|
| `c2` |SMPT Timeout|0.00|N/A|
| `c3` |Serializable|2.08|✅|
| `c4` |SMPT Timeout|1.20|N/A|
| `c5` |Not serializable|0.50|✅|
| `c6` |Not serializable|0.37|✅|
| `c7` |Not serializable|0.49|✅|
| `c8` |SMPT Timeout|1.25|N/A|
| `d1` |Serializable|3.58|✅|
| `d2` |Not serializable|0.57|✅|
| `d3` |Serializable|5.38|✅|
| `d4` |Serializable|8.95|✅|
| `d5` |Not serializable|0.32|✅|
| `e1` |Serializable|0.45|✅|
| `e2` |SMPT Timeout|0.69|N/A|
| `e3` |SMPT Timeout|3.90|N/A|
| `e4` |SMPT Timeout|5.05|N/A|
| `e5` |Serializable|0.10|✅|
| `e6` |Serializable|0.15|✅|
| `e7` |Serializable|0.46|✅|
| `f1` |Serializable|11.67|✅|
| `f2` |Not serializable|0.35|✅|
| `f3` |Not serializable|0.36|✅|
| `f4` |Serializable|8.86|✅|
| `f5` |SMPT Timeout|0.00|N/A|
| `f6` |Not serializable|129.85|✅|
| `f7` |Not serializable|0.40|✅|
| `f8` |SMPT Timeout|0.31|N/A|
| `f9` |Serializable|0.12|✅|
| `g1` |SMPT Timeout|14.92|N/A|
| `g2` |Error|52.07|N/A|
| `g3` |SMPT Timeout|0.00|N/A|
| `g4` |Not serializable|0.97|✅|
| `g5` |Error|41.66|N/A|
| `g6` |Not serializable|5.41|✅|
| `g7` |Serializable|11.22|✅|

## Summary
- Serializable: 17 (valid proofs: 17, invalid: 0)
- Not serializable: 13 (valid traces: 13, invalid: 0)
- Timeouts: 15, Errors: 2, Total: 47
