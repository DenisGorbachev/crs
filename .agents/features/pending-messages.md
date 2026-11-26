# Queued and steering messages

- Keep pending messages in host state (`ChatState.queuedMessages`).
- While a turn is active, offer **Queue** and **Steer** for a submission.
  - **Queue**: the host starts the message as a new turn after the active turn ends.
  - **Steer**: the host injects the message into the active turn.
- Show pending messages with steering first, then queued, each group in host order.
- For each queued message, offer **Steer**, **Edit**, **Cancel**.
- For each steering message, offer **Cancel**.
- If the turn ends before the host consumes a steering message, keep it pending and offer **Queue** to send it as a new turn.
- Use `chat/pendingMessageSet` (new ID to add, same ID to edit or change kind) and `chat/pendingMessageRemoved`.
