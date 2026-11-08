<!--
Expected findings:
Document:
- line None: INFO CHECK_COVERAGE
Default configuration (patterns disabled):
- 0 errors, 0 warnings, 1 info (CHECK_COVERAGE)
When pattern reports enabled:
- 1.1: INFO EARS_PATTERN pattern: ubiquitous
- 1.2: INFO EARS_PATTERN pattern: event-driven
- 2.1: INFO EARS_PATTERN pattern: state-driven
- 2.2: INFO EARS_PATTERN pattern: unwanted-behaviour
- 3.1: INFO EARS_PATTERN pattern: optional-feature
- 3.2: INFO EARS_PATTERN pattern: complex
-->

# Requirements Document

## Introduction

Known-good requirements document satisfying all EARS and constraint checks.

## Requirements

### Requirement 1: Telemetry and mode control

**Objective:** The controller tracks telemetry and switches modes.

_Terms: mode, depth, armed_

#### Acceptance Criteria

1. The controller shall record all telemetry readings.
   _Guard: mode = normal_
   _Effect: armed = false_

2. When the on-call engineer signals, the controller shall switch to safe mode.
   _Terms: operator_
   _Guard: mode = normal_
   _Effect: armed = false_

### Requirement 2: Safety mechanism

**Objective:** The controller manages the safety mechanism.

_Terms: mode, armed_

#### Acceptance Criteria

1. While mode is safe, the controller shall keep the mechanism armed.
   _Guard: mode = safe_
   _Effect: armed = true_
   _Conflicts: 2.2_

2. If depth is zero, then the controller shall keep the mechanism disarmed.
   _Terms: depth_
   _Guard: mode = maintenance_
   _Effect: armed = false_

### Requirement 3: Operator interaction

**Objective:** The controller coordinates with the operator.

_Terms: mode, operator, depth_

#### Acceptance Criteria

1. Where operator is configured, the controller shall notify the operator.

2. While mode is safe and when operator is present, the controller shall verify the depth.
