# Review system

## Goal

Build a system to support the following workflow:

- User writes a message
- User sends a message to the agent
  - If the message is associated with an old agent thread:
    - Then: it is sent to the old thread
    - Else: it is sent to the new thread
- Agent processes the user message
  - Agent may edit files
  - Agent may run commands
    - Commands may modify external state (e.g. deploy)
- Agent sends a message to the user
- User reviews the agent message
  - The final message
  - The modified files
  - The external state
- If the review is empty:
  - Then: done
  - Else: user sends the review as a message to the agent

## Preferred tools

- iTerm2
- ZSH
- tmux

I'm open to switching to other tools.

## Ideas

- Use a ZLE widget to launch a child shell
- Ask the agent to run a command to explicitly mark files or repos for review
