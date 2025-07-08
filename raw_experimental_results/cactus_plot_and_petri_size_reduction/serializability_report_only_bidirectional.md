# Serializability Analysis Report - _Only_Bidirectional
Generated: 2025-07-03 19:58:06
Extras: ['--without-remove-redundant', '--without-generate-less', '--without-smart-kleene-order']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.71|✅|
| `b2` |SMPT Timeout|0.14|N/A|
| `b3` |SMPT Timeout|0.08|N/A|
| `b4` |SMPT Timeout|0.09|N/A|
| `a1` |Serializable|0.08|✅|
| `a2` |Not serializable|0.29|✅|
| `a3` |Serializable|0.08|✅|
| `a4` |Serializable|0.89|✅|
| `a5` |SMPT Timeout|0.09|N/A|
| `a6` |Not serializable|0.49|✅|
| `a7` |Serializable|0.09|✅|
| `c1` |SMPT Timeout|0.00|N/A|
| `c2` |SMPT Timeout|0.00|N/A|
| `c3` |Serializable|1.71|✅|
| `c4` |Serializable|5.56|✅|
| `c5` |Not serializable|0.42|✅|
| `c6` |Not serializable|0.31|✅|
| `c7` |Not serializable|0.44|✅|
| `c8` |Serializable|6.87|✅|
| `d1` |Serializable|4.49|✅|
| `d2` |Not serializable|0.37|✅|
| `d3` |Serializable|28.08|✅|
| `d4` |Serializable|14.95|✅|
| `d5` |Not serializable|0.32|✅|
| `e1` |Serializable|0.41|✅|
| `e2` |SMPT Timeout|0.46|N/A|
| `e3` |Not serializable|2.58|✅|
| `e4` |SMPT Timeout|3.07|N/A|
| `e5` |Serializable|0.11|✅|
| `e6` |Serializable|0.16|✅|
| `e7` |Serializable|0.41|✅|
| `f1` |Serializable|0.37|✅|
| `f2` |Not serializable|0.35|✅|
| `f3` |Not serializable|0.35|✅|
| `f4` |Serializable|8.22|✅|
| `f5` |SMPT Timeout|0.00|N/A|
| `f6` |Not serializable|113.12|✅|
| `f7` |Not serializable|0.31|✅|
| `f8` |Not serializable|0.49|✅|
| `f9` |Serializable|0.12|✅|
| `g1` |SMPT Timeout|14.33|N/A|
| `g2` |Error|123.17|N/A|
| `g3` |SMPT Timeout|0.00|N/A|
| `g4` |Not serializable|0.72|✅|
| `g5` |Serializable|10.40|✅|
| `g6` |Not serializable|5.48|✅|
| `g7` |Serializable|259.22|✅|

## Summary
- Serializable: 20 (valid proofs: 20, invalid: 0)
- Not serializable: 15 (valid traces: 15, invalid: 0)
- Timeouts: 11, Errors: 1, Total: 47
