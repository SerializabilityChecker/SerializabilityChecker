# Serializability Analysis Report
Generated: 2025-07-03 21:03:02
Extras: []

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.63|✅|
| `b2` |Serializable|6.11|✅|
| `b3` |Serializable|1.51|✅|
| `b4` |Serializable|1.50|✅|
| `a1` |Serializable|0.08|✅|
| `a2` |Not serializable|0.26|✅|
| `a3` |Serializable|0.08|✅|
| `a4` |Serializable|0.80|✅|
| `a5` |Serializable|10.25|✅|
| `a6` |Not serializable|0.44|✅|
| `a7` |Serializable|0.09|✅|
| `c1` |SMPT Timeout|0.48|N/A|
| `c2` |Serializable|242.39|✅|
| `c3` |Serializable|1.58|✅|
| `c4` |Serializable|4.62|✅|
| `c5` |Not serializable|0.34|✅|
| `c6` |Not serializable|0.31|✅|
| `c7` |Not serializable|0.37|✅|
| `c8` |Serializable|5.72|✅|
| `d1` |Serializable|3.91|✅|
| `d2` |Not serializable|0.34|✅|
| `d3` |Serializable|8.04|✅|
| `d4` |Serializable|16.69|✅|
| `d5` |Not serializable|0.28|✅|
| `e1` |Serializable|0.40|✅|
| `e2` |SMPT Timeout|0.41|N/A|
| `e3` |Not serializable|1.18|✅|
| `e4` |SMPT Timeout|1.55|N/A|
| `e5` |Serializable|0.09|✅|
| `e6` |Serializable|0.15|✅|
| `e7` |Serializable|0.37|✅|
| `f1` |Serializable|0.32|✅|
| `f2` |Not serializable|0.32|✅|
| `f3` |Not serializable|0.45|✅|
| `f4` |Serializable|7.50|✅|
| `f5` |Serializable|7.12|✅|
| `f6` |Not serializable|0.44|✅|
| `f7` |Not serializable|0.27|✅|
| `f8` |Not serializable|0.49|✅|
| `f9` |Serializable|0.11|✅|
| `g1` |Not serializable|16.98|✅|
| `g2` |SMPT Timeout|2.41|N/A|
| `g3` |Not serializable|2.03|✅|
| `g4` |Not serializable|0.64|✅|
| `g5` |Serializable|9.08|✅|
| `g6` |Not serializable|4.67|✅|
| `g7` |Serializable|211.28|✅|

## Summary
- Serializable: 26 (valid proofs: 26, invalid: 0)
- Not serializable: 17 (valid traces: 17, invalid: 0)
- Timeouts: 4, Errors: 0, Total: 47
