# Serializability Analysis Report
Generated: 2025-07-03 21:18:10
Extras: ['--without-generate-less']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.59|✅|
| `b2` |SMPT Timeout|0.18|N/A|
| `b3` |SMPT Timeout|0.08|N/A|
| `b4` |SMPT Timeout|0.08|N/A|
| `a1` |Serializable|0.08|✅|
| `a2` |Not serializable|0.23|✅|
| `a3` |Serializable|0.09|✅|
| `a4` |Serializable|0.77|✅|
| `a5` |SMPT Timeout|0.08|N/A|
| `a6` |Not serializable|0.45|✅|
| `a7` |Serializable|0.08|✅|
| `c1` |SMPT Timeout|1.06|N/A|
| `c2` |SMPT Timeout|0.00|N/A|
| `c3` |Serializable|1.57|✅|
| `c4` |Serializable|5.07|✅|
| `c5` |Not serializable|0.32|✅|
| `c6` |Not serializable|0.28|✅|
| `c7` |Not serializable|0.38|✅|
| `c8` |Serializable|6.23|✅|
| `d1` |Serializable|3.99|✅|
| `d2` |Not serializable|0.36|✅|
| `d3` |Serializable|5.85|✅|
| `d4` |Serializable|13.85|✅|
| `d5` |Not serializable|0.27|✅|
| `e1` |Serializable|0.37|✅|
| `e2` |SMPT Timeout|0.40|N/A|
| `e3` |Not serializable|2.60|✅|
| `e4` |SMPT Timeout|2.96|N/A|
| `e5` |Serializable|0.10|✅|
| `e6` |Serializable|0.14|✅|
| `e7` |Serializable|0.41|✅|
| `f1` |Serializable|0.34|✅|
| `f2` |Not serializable|0.33|✅|
| `f3` |Not serializable|0.36|✅|
| `f4` |Serializable|8.06|✅|
| `f5` |SMPT Timeout|0.00|N/A|
| `f6` |Not serializable|97.34|✅|
| `f7` |Not serializable|0.31|✅|
| `f8` |Not serializable|0.49|✅|
| `f9` |Serializable|0.13|✅|
| `g1` |SMPT Timeout|13.96|N/A|
| `g2` |Error|142.28|N/A|
| `g3` |SMPT Timeout|0.00|N/A|
| `g4` |Not serializable|0.74|✅|
| `g5` |Serializable|9.55|✅|
| `g6` |Not serializable|5.25|✅|
| `g7` |SMPT Timeout|0.00|N/A|

## Summary
- Serializable: 19 (valid proofs: 19, invalid: 0)
- Not serializable: 15 (valid traces: 15, invalid: 0)
- Timeouts: 12, Errors: 1, Total: 47
