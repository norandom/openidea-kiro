<!--
Expected findings:
Requirement 1 (v1 regression false positives - NO findings expected):
- 1.1: (no findings: quoted modal "must" inside quotes)
- 1.2: (no findings: hyphenated modal in must-gather)
- 1.3: (no findings: possessive apostrophe in tree's)
- 1.4: (no findings: parenthetical comma in response after shall)
- 1.5: (no findings: If form without then)
- 1.6: (no findings: If form with then)
Requirement 2 (true positive findings):
- 2.1: ERROR EARS_NO_SHALL criterion has no shall
- 2.2: WARNING EARS_MULTI_SHALL found 2
- 2.3: WARNING EARS_MODAL use shall, not must
- 2.4: WARNING EARS_NO_TRIGGER condition keyword without comma before shall
- 2.5: WARNING EARS_UNPARSED no subject before shall
- 2.6: WARNING EARS_UNPARSED no response after shall
-->

# Requirements Document

## Introduction

Corpus of EARS regression criteria verifying version 1 false positive fixes and true positive detection.

## Requirements

### Requirement 1: Version 1 false positive regressions

**Objective:** Verify that literals, hyphenated words, possessives, and response commas do not trigger false findings.

#### Acceptance Criteria

1. The system shall display "you must enter a password".

2. The tool shall run the must-gather diagnostic script.

3. The validator shall verify the tree's root hash.

4. The system shall send the email, which was queued, to the user.

5. If the input is empty, the parser shall return an error.

6. If the input is empty, then the parser shall return an error.

### Requirement 2: True positive EARS errors and warnings

**Objective:** Verify that true EARS defects are correctly identified with their codes.

#### Acceptance Criteria

1. The controller records all telemetry data.

2. The controller shall record telemetry and the controller shall emit a pulse.

3. The controller must verify the operating parameters.

4. When system is ready the controller shall start.

5. When ready, shall process incoming messages.

6. When ready, the controller shall.
