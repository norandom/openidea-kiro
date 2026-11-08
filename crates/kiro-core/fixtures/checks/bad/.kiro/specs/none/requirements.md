<!--
Expected findings:
Document:
- line 1: WARNING TOPICS_UNDECLARED spec declares no topics in spec.json
- line 19: WARNING CHECK_UNSCOPED annotations present but not checked: spec declares no topics
(no CHECK_COVERAGE because neither term nor contradiction check runs)
-->

# Requirements Document

## Introduction

Unscoped requirements document with annotations present but spec declares no topics.

## Requirements

### Requirement 1: Unscoped operation

**Objective:** Demonstrate CHECK_UNSCOPED when annotations are present without declared topics.

_Terms: mode_

#### Acceptance Criteria

1. The controller shall maintain operational parameters.
   _Guard: mode = safe_
