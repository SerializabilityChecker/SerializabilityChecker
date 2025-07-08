# Serializability Analysis Report - _Only_Generate_Less
Generated: 2025-07-03 20:06:43
Extras: ['--without-bidirectional', '--without-remove-redundant', '--without-smart-kleene-order']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.78|✅|
| `b2` |SMPT Timeout|0.09|N/A|
| `b3` |Serializable|3.10|✅|
| `b4` |Serializable|2.67|✅|
| `a1` |Serializable|0.09|✅|
| `a2` |Not serializable|0.29|✅|
| `a3` |Serializable|0.09|✅|
| `a4` |Serializable|1.22|✅|
| `a5` |Serializable|65.04|✅|
| `a6` |Not serializable|0.49|✅|
| `a7` |Serializable|0.10|✅|
| `c1` |SMPT Timeout|0.88|N/A|
| `c2` |SMPT Timeout|1.60|N/A|
| `c3` |Serializable|2.09|✅|
| `c4` |SMPT Timeout|1.14|N/A|
| `c5` |Not serializable|0.44|✅|
| `c6` |Not serializable|0.35|✅|
| `c7` |Not serializable|0.44|✅|
| `c8` |SMPT Timeout|1.20|N/A|
| `d1` |Serializable|3.08|✅|
| `d2` |Not serializable|0.36|✅|
| `d3` |Serializable|6.35|✅|
| `d4` |Serializable|7.36|✅|
| `d5` |Not serializable|0.30|✅|
| `e1` |Serializable|0.44|✅|
| `e2` |SMPT Timeout|0.64|N/A|
| `e3` |SMPT Timeout|2.07|N/A|
| `e4` |SMPT Timeout|3.28|N/A|
| `e5` |Serializable|0.11|✅|
| `e6` |Serializable|0.16|✅|
| `e7` |Serializable|0.45|✅|
| `f1` |Serializable|11.26|✅|
| `f2` |Not serializable|0.35|✅|
| `f3` |Not serializable|0.34|✅|
| `f4` |Serializable|12.58|✅|
| `f5` |Serializable|7.87|✅|
| `f6` |Not serializable|0.50|✅|
| `f7` |Not serializable|0.31|✅|
| `f8` |SMPT Timeout|0.31|N/A|
| `f9` |Serializable|0.12|✅|
| `g1` |SMPT Timeout|22.15|N/A|
| `g2` |SMPT Timeout|5.97|N/A|
| `g3` |SMPT Timeout|5.99|N/A|
| `g4` |Not serializable|0.94|✅|
| `g5` |Serializable|42.35|✅|
| `g6` |Not serializable|5.21|✅|
| `g7` |Serializable|11.28|✅|

## Summary
- Serializable: 22 (valid proofs: 22, invalid: 0)
- Not serializable: 13 (valid traces: 13, invalid: 0)
- Timeouts: 12, Errors: 0, Total: 47
