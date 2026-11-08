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

_Terms: 

#### Acceptance Criteria

1. The controller shall maintain wide configuration.
   _Guard: 
   _Conflicts: 1.2_

2. The controller shall alter wide configuration.
   _Guard: 