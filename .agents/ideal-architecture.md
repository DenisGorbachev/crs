# Ideal architecture

## Definitions

### UI client

### UI sync server

Notes:

- UI sync server is needed only to synchronize the message drafts. Maybe it is possible for the agent server to implement `acquire` method for message draft (it would force-set the UI client ID that is currently editing the message draft and send back the current state of the message draft, so that the acquiring client could "resume" editing and the releasing client would get notified that its lock was acquired by another client - the UI could then show it to the user).

## Agent server

Agent host examples:

- A remote server.
- A sandbox running in Docker on a user laptop.
- A user laptop.
