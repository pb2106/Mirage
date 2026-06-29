# Mirage Session Shell

The `mirage shell` command allows you to drop into an interactive bash shell that is already sandboxed with a given Mirage profile.

## How Linux Namespace Inheritance Works

Linux namespaces (like those used by Mirage to sandbox applications) are inherited automatically by child processes. When you run `mirage shell --profile <name>`, you get a shell inside the sandboxed namespaces. Any process you start from that shell—whether it's a CLI tool, a web browser, or an editor—will automatically be a child of that shell and will therefore inherit the same namespace and spoofed identity.

## The GUI Terminal Emulator Sharp Edge

A common pitfall occurs when using modern GUI terminal emulators (like GNOME Terminal, Konsole, or Kitty). If you have a Mirage shell open in one tab and you click the "New Tab" or "New Window" button in your terminal emulator, **the new tab will NOT be inside the sandbox**.

This happens because the "New Tab" button sends a request to the GUI terminal application (which runs on the host, outside the sandbox) to fork a fresh shell. That new shell is a child of the terminal emulator, not a child of your sandboxed shell, so it runs in the host's default namespace. This means you could inadvertently leak your real identity in the new tab!

*(Note: This is standard Linux namespace behavior and works the same way with Docker dev containers and Firejail. It is not a bug in Mirage.)*

## Recommended Workarounds

To safely multitask within a sandboxed session, you have two options:

1. **Use `tmux`**: Run `mirage shell --profile <name> --tmux`. This launches a tmux server inside the sandbox. When you create new tmux panes or windows (e.g., via `Ctrl-B %`), they are forked directly by the sandboxed tmux server and perfectly inherit the namespaces.
2. **Launch a new session**: Open a new terminal tab normally, and run `mirage shell --profile <name>` again in that new tab to start a second, separate sandbox using the same profile.
