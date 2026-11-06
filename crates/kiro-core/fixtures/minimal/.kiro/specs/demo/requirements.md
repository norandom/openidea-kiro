# Requirements Document

## Introduction

Fixture requirements in the canonical kiro grammar, with annotations at the
requirement and the criterion level.

## Requirements

### Requirement 1: Own a file

**Objective:** As a developer, I want the component that owns a file, so that I can navigate without reading the whole design.

_Terms: spec, requirement_

#### Acceptance Criteria

1. The demo engine shall report the component that owns a project-relative file.
2. While a file is mapped by exactly one component, the demo engine shall report exactly that component.
   - _Terms: component_
   - _Guard: mode = safe_

### Requirement 2: Count tasks

**Objective:** As a reviewer, I want task counts, so that I can see progress at a glance.

#### Acceptance Criteria

1. The demo engine shall count majors and sub-tasks.
2. The demo engine shall mark completed tasks.
