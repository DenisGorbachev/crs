# crs

## Decisions

- How to deal with file-level changes that are not tied to a specific code item?
  - Examples:
    - Adding `#![no_std]`
- How to deal with inter-dependent code items?

## crs package

### ShowCommand

- Must have methods:
  - `run`
    - Must find the first review item that is not approved but whose dependencies are approved
      - Must descend into the first unapproved unseen dependency
        - Notes:
          - The "unseen" check is needed because two Rust code items can be inter-dependent
