# Requirements Document

## Introduction

Telemetry controller requirements.

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
