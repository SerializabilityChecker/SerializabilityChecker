# Serializability Analysis Report
Generated: 2025-07-03 21:07:49
Extras: ['--without-remove-redundant']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.62|✅|
| `b2` |SMPT Timeout|0.08|N/A|
| `b3` |Serializable|2.90|✅|
| `b4` |Serializable|2.24|✅|
| `a1` |Serializable|0.07|✅|
| `a2` |Not serializable|0.26|✅|
| `a3` |Serializable|0.08|✅|
| `a4` |Serializable|0.77|✅|
| `a5` |Serializable|24.31|✅|
| `a6` |Not serializable|0.42|✅|
| `a7` |Serializable|0.09|✅|
| `c1` |SMPT Timeout|0.51|N/A|
| `c2` |Serializable|241.94|✅|
| `c3` |Serializable|1.60|✅|
| `c4` |Serializable|4.87|✅|
| `c5` |Not serializable|0.35|✅|
| `c6` |Not serializable|0.32|✅|
| `c7` |Not serializable|0.59|✅|
| `c8` |Serializable|5.78|✅|
| `d1` |Serializable|4.01|✅|
| `d2` |Not serializable|0.34|✅|
| `d3` |Serializable|8.24|✅|
| `d4` |Serializable|17.40|✅|
| `d5` |Not serializable|0.28|✅|
| `e1` |Serializable|0.38|✅|
| `e2` |SMPT Timeout|0.38|N/A|
| `e3` |Not serializable|1.09|✅|
| `e4` |SMPT Timeout|1.43|N/A|
| `e5` |Serializable|0.10|✅|
| `e6` |Serializable|0.15|✅|
| `e7` |Serializable|0.39|✅|
| `f1` |Serializable|0.31|✅|
| `f2` |Not serializable|0.31|✅|
| `f3` |Not serializable|0.46|✅|
| `f4` |Serializable|7.76|✅|
| `f5` |Serializable|10.81|✅|
| `f6` |Not serializable|0.47|✅|
| `f7` |Not serializable|0.28|✅|
| `f8` |Not serializable|0.48|✅|
| `f9` |Serializable|0.12|✅|
| `g1` |Not serializable|20.52|✅|
| `g2` |SMPT Timeout|5.45|N/A|
| `g3` |Not serializable|1.80|✅|
| `g4` |Not serializable|0.63|✅|
| `g5` |Serializable|9.37|✅|
| `g6` |Not serializable|5.06|✅|
| `g7` |Serializable|212.60|✅|

## Summary
- Serializable: 25 (valid proofs: 25, invalid: 0)
- Not serializable: 17 (valid traces: 17, invalid: 0)
- Timeouts: 5, Errors: 0, Total: 47
