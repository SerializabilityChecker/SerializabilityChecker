# Serializability Analysis Report
Generated: 2025-07-03 21:11:57
Extras: ['--without-smart-kleene-order']

|Example|Result|CPU(s)|Valid?|
|--|--|--|--|
| `b1` |Serializable|0.63|✅|
| `b2` |Serializable|6.58|✅|
| `b3` |Serializable|1.56|✅|
| `b4` |Serializable|1.56|✅|
| `a1` |Serializable|0.07|✅|
| `a2` |Not serializable|0.24|✅|
| `a3` |Serializable|0.08|✅|
| `a4` |Serializable|0.80|✅|
| `a5` |Serializable|11.00|✅|
| `a6` |Not serializable|0.44|✅|
| `a7` |Serializable|0.08|✅|
| `c1` |SMPT Timeout|0.47|N/A|
| `c2` |Serializable|99.65|✅|
| `c3` |Serializable|1.58|✅|
| `c4` |Serializable|5.09|✅|
| `c5` |Not serializable|0.32|✅|
| `c6` |Not serializable|0.30|✅|
| `c7` |Not serializable|0.38|✅|
| `c8` |Serializable|6.23|✅|
| `d1` |Serializable|4.20|✅|
| `d2` |Not serializable|0.36|✅|
| `d3` |Serializable|8.58|✅|
| `d4` |Serializable|18.09|✅|
| `d5` |Not serializable|0.29|✅|
| `e1` |Serializable|0.41|✅|
| `e2` |SMPT Timeout|0.42|N/A|
| `e3` |Not serializable|1.26|✅|
| `e4` |SMPT Timeout|1.70|N/A|
| `e5` |Serializable|0.11|✅|
| `e6` |Serializable|0.15|✅|
| `e7` |Serializable|0.40|✅|
| `f1` |Serializable|0.35|✅|
| `f2` |Not serializable|0.35|✅|
| `f3` |Not serializable|0.35|✅|
| `f4` |Serializable|8.27|✅|
| `f5` |Serializable|7.31|✅|
| `f6` |Not serializable|0.50|✅|
| `f7` |Not serializable|0.33|✅|
| `f8` |Not serializable|0.49|✅|
| `f9` |Serializable|0.13|✅|
| `g1` |Not serializable|16.36|✅|
| `g2` |SMPT Timeout|2.53|N/A|
| `g3` |Not serializable|2.17|✅|
| `g4` |Not serializable|0.70|✅|
| `g5` |Serializable|9.11|✅|
| `g6` |Not serializable|5.09|✅|
| `g7` |Serializable|199.63|✅|

## Summary
- Serializable: 26 (valid proofs: 26, invalid: 0)
- Not serializable: 17 (valid traces: 17, invalid: 0)
- Timeouts: 4, Errors: 0, Total: 47
