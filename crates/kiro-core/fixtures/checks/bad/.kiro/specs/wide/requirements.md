<!--
Expected findings:
Document:
- line None: INFO CHECK_COVERAGE
- line None: WARNING SEARCH_REFUSED variable cap 512 exceeded: 520 values; 1 pairs not checked: 1.1/1.2; largest terms: w01 (8 values), w02 (8 values)
Criteria:
- 1.1: (refused pair with 1.2, no CONTRADICTION)
- 1.2: (refused pair with 1.1)
-->

# Requirements Document

## Introduction

Wide requirements document exceeding default variable cap of 512.

## Requirements

### Requirement 1: Wide model

**Objective:** Exceed the variable cap with 65 enum terms of 8 values each.

_Terms: w01, w02, w03, w04, w05, w06, w07, w08, w09, w10, w11, w12, w13, w14, w15, w16, w17, w18, w19, w20, w21, w22, w23, w24, w25, w26, w27, w28, w29, w30, w31, w32, w33, w34, w35, w36, w37, w38, w39, w40, w41, w42, w43, w44, w45, w46, w47, w48, w49, w50, w51, w52, w53, w54, w55, w56, w57, w58, w59, w60, w61, w62, w63, w64, w65_

#### Acceptance Criteria

1. The controller shall maintain wide configuration.
   _Guard: w01 = v1 and w02 = v1 and w03 = v1 and w04 = v1 and w05 = v1 and w06 = v1 and w07 = v1 and w08 = v1 and w09 = v1 and w10 = v1 and w11 = v1 and w12 = v1 and w13 = v1 and w14 = v1 and w15 = v1 and w16 = v1 and w17 = v1 and w18 = v1 and w19 = v1 and w20 = v1 and w21 = v1 and w22 = v1 and w23 = v1 and w24 = v1 and w25 = v1 and w26 = v1 and w27 = v1 and w28 = v1 and w29 = v1 and w30 = v1 and w31 = v1 and w32 = v1 and w33 = v1 and w34 = v1 and w35 = v1 and w36 = v1 and w37 = v1 and w38 = v1 and w39 = v1 and w40 = v1 and w41 = v1 and w42 = v1 and w43 = v1 and w44 = v1 and w45 = v1 and w46 = v1 and w47 = v1 and w48 = v1 and w49 = v1 and w50 = v1 and w51 = v1 and w52 = v1 and w53 = v1 and w54 = v1 and w55 = v1 and w56 = v1 and w57 = v1 and w58 = v1 and w59 = v1 and w60 = v1 and w61 = v1 and w62 = v1 and w63 = v1 and w64 = v1 and w65 = v1_
   _Conflicts: 1.2_

2. The controller shall alter wide configuration.
   _Guard: w01 = v2_
