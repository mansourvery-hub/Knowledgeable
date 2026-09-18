# Knowledgeable: LLM Streaming & Backend Integration Log

## Overview
This document tracks the resolution of critical streaming stability issues and backend integration failures for the Knowledgeable AI tutoring app.

## 1. Critical Performance Fixes (Client)
The application originally suffered from uncontrollable memory and CPU spikes (OOM crashes) during Gemini responses.

### Issues Resolved:
*   **Unbounded Recursive Loop**: `sse_stream_web.dart` used a recursively invoked `readChunk()` function in JS interop. Unexpected reader states created an infinite, unbounded microtask loop.
*   **Pathological State Churn**: `ChatController` updated the `ChatState` with the entire accumulated response string on every token delta.
*   **Resource Leaks**: The JS `ReadableStreamDefaultReader` was not explicitly cancelled, leading to orphaned background processes.

### Implementation:
*   **SSE Pump**: Replaced recursion with an iterative `while` loop, including timeouts and explicit `reader.cancel()` calls for robust lifecycle management.
*   **Delta Buffering**: Introduced a 33ms timer-based throttle buffer for assistant messages, decoupling token generation frequency from Flutter/Riverpod UI update frequency.
*   **Invalidation**: Consolidated `messagesProvider` invalidation to trigger only on explicit `turn_completed` events.

## 2. Backend Integration & Authentication Fixes
After stabilizing the client, the LLM integration remained non-functional due to authentication and provider configuration issues.

### Issues Resolved:
*   **CORS Blockers**: The browser was blocking frontend-to-backend communication due to strict CORS policy enforcement.
*   **Invalid Model Configuration**: The backend was configured to use `gemini-1.5-flash`, which was returning `404 NOT_FOUND` for the configured endpoint.
*   **Authentication**: Environment variable propagation was inconsistent, leading to `403 PERMISSION_DENIED` errors when querying the Gemini provider.

### Implementation:
*   **Enhanced CORS**: Updated `crates/api/src/main.rs` to include `OPTIONS` methods and exposed relevant headers (`Content-Type`, `Authorization`) in the `CorsLayer`.
*   **Model Validation**: Used direct REST calls to list supported models via `generativelanguage.googleapis.com` to identify `gemini-3.5-flash` as a stable, supported model.
*   **Environment Configuration**: Updated `GEMINI_MODEL` to `gemini-3.5-flash` and verified environment variable propagation in the backend process environment.
*   **Instrumented Telemetry**: Added lifecycle `dev.log` points (fetch status, body receipt, byte emission) to allow real-time debugging of the SSE pipeline in browser devtools.

## 3. Verified Wire Protocol
The backend successfully streams using the expected SSE format:
```text
event: text_delta
data: {"version":1,"event":"text_delta","data":{"text":"..."}}
...
event: turn_completed
data: {"version":1,"event":"turn_completed","data":{}}
```
The parser `parseSseStream` has been verified to correctly handle these frames.
