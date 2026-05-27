"use client";

import { create } from "zustand";
import { streamUrl } from "./api";
import type { RequestEvent } from "./types";

/** How many recent requests to keep on screen. */
const MAX_POINTS = 400;

interface StreamState {
  times: number[]; // unix seconds
  latencies: number[]; // ms
  failures: (number | null)[]; // latency where the request failed, else null
  connected: boolean;
  connect: () => void;
}

// Module-scoped so React's double-mount (strict mode) reuses one socket.
let socket: WebSocket | null = null;

export const useStream = create<StreamState>()((set, get) => ({
  times: [],
  latencies: [],
  failures: [],
  connected: false,

  connect: () => {
    if (socket || typeof window === "undefined") return;

    const open = () => {
      socket = new WebSocket(streamUrl());
      socket.onopen = () => set({ connected: true });
      socket.onmessage = (message) => {
        const event = JSON.parse(message.data) as RequestEvent;
        if (event.kind !== "request") return;
        const { times, latencies, failures } = get();
        set({
          times: [...times, event.timestamp_ns / 1e9].slice(-MAX_POINTS),
          latencies: [...latencies, event.duration_ms].slice(-MAX_POINTS),
          failures: [...failures, event.failed ? event.duration_ms : null].slice(-MAX_POINTS),
        });
      };
      socket.onclose = () => {
        set({ connected: false });
        socket = null;
        setTimeout(open, 1500); // auto-reconnect
      };
      socket.onerror = () => socket?.close();
    };

    open();
  },
}));
