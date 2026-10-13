# `src-tauri/capabilities` — Security & Access Control Declarations

## Overview

The `src-tauri/capabilities` directory contains Tauri v2 capability configuration files that define security access control policies for webview windows.

---

## File Manifest

| File | Purpose | Key Permissions Declared |
|---|---|---|
| [`default.json`](default.json) | Application window permission definitions | Configures allowed permissions for the primary webview window (`main`): core event dispatching, `tauri-plugin-opener` (for safely launching job links in the default browser), `tauri-plugin-notification` (for desktop crawl alerts), and sidecar execution privileges. |

---

## Security Model

Under Tauri v2, capabilities follow a least-privilege security model. The frontend cannot execute shell commands or access native APIs unless explicitly declared in capability sets.
