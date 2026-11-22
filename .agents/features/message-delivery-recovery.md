# Message delivery recovery

- Preserve the exact submitted text and its destination across disconnects and restarts until delivery is confirmed or the user resolves it.
- After reconnecting, match each pending submission against the agent's history or acknowledgement.
- If delivery cannot be determined, show **Delivery unknown** with the recoverable text. Resend only at the user's request.
