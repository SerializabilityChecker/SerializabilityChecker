# Serializability Analysis Report - _Only_Smart_Kleene
Generated: 2025-07-03 20:12:03
Extras: ['--without-bidirectional', '--without-remove-redundant', '--without-generate-less']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.79|✅|
| `b2` |SMPT Timeout|0.17|N/A|
| `b3` |SMPT Timeout|0.09|N/A|
| `b4` |SMPT Timeout|0.10|N/A|
| `a1` |Serializable|0.08|✅|
| `a2` |Not serializable|0.28|✅|
| `a3` |Serializable|0.09|✅|
| `a4` |Serializable|1.19|✅|
| `a5` |SMPT Timeout|0.09|N/A|
| `a6` |Not serializable|0.50|✅|
| `a7` |Serializable|0.11|✅|
| `c1` |SMPT Timeout|1.36|N/A|
| `c2` |SMPT Timeout|2.08|N/A|
| `c3` |Serializable|2.10|✅|
| `c4` |SMPT Timeout|1.07|N/A|
| `c5` |Not serializable|0.42|✅|
| `c6` |Not serializable|0.35|✅|
| `c7` |Not serializable|0.45|✅|
| `c8` |SMPT Timeout|1.17|N/A|
| `d1` |Serializable|3.27|✅|
| `d2` |Not serializable|0.35|✅|
| `d3` |Serializable|5.22|✅|
| `d4` |Serializable|8.26|✅|
| `d5` |Not serializable|0.31|✅|
| `e1` |Serializable|0.45|✅|
| `e2` |SMPT Timeout|0.69|N/A|
| `e3` |SMPT Timeout|3.62|N/A|
| `e4` |SMPT Timeout|4.30|N/A|
| `e5` |Serializable|0.10|✅|
| `e6` |Serializable|0.15|✅|
| `e7` |Serializable|0.45|✅|
| `f1` |Serializable|11.45|✅|
| `f2` |Not serializable|0.32|✅|
| `f3` |Not serializable|0.35|✅|
| `f4` |Serializable|12.68|✅|
| `f5` |SMPT Timeout|0.00|N/A|
| `f6` |Not serializable|121.70|✅|
| `f7` |Not serializable|0.30|✅|
| `f8` |SMPT Timeout|0.32|N/A|
| `f9` |Serializable|0.14|✅|
| `g1` |SMPT Timeout|14.71|N/A|
| `g2` |Error|43.44|N/A|
| `g3` |SMPT Timeout|0.00|N/A|
| `g4` |Not serializable|0.94|✅|
| `g5` |Serializable|43.61|✅|
| `g6` |Not serializable|5.09|✅|
| `g7` |Serializable|11.44|✅|

## Summary
- Serializable: 18 (valid proofs: 18, invalid: 0)
- Not serializable: 13 (valid traces: 13, invalid: 0)
- Timeouts: 15, Errors: 1, Total: 47
