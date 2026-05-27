"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const NAV = [
  { href: "/", label: "Overview", ready: true },
  { href: "/map", label: "Service map", ready: true },
  { href: "/traces", label: "Traces", ready: true },
  { href: "/anomalies", label: "Anomalies", ready: false },
  { href: "/query", label: "Query", ready: false },
  { href: "/logs", label: "Logs", ready: false },
];

export function Sidebar() {
  const path = usePathname();
  return (
    <aside className="flex w-56 shrink-0 flex-col gap-9 border-r border-line px-5 py-6">
      <div className="flex items-center gap-2.5">
        <span className="text-lg leading-none text-teal">◉</span>
        <span className="font-display text-[17px] font-semibold tracking-tight">argus</span>
      </div>

      <nav className="flex flex-col gap-0.5 text-[13px]">
        {NAV.map((item) =>
          item.ready ? (
            <Link
              key={item.href}
              href={item.href}
              className={`rounded-md px-3 py-2 transition-colors ${
                path === item.href ? "bg-elevated text-text" : "text-muted hover:bg-elevated/50 hover:text-text"
              }`}
            >
              {item.label}
            </Link>
          ) : (
            <span key={item.href} className="flex items-center justify-between rounded-md px-3 py-2 text-faint">
              {item.label}
              <span className="font-mono text-[9px] uppercase tracking-widest opacity-70">soon</span>
            </span>
          ),
        )}
      </nav>

      <div className="mt-auto flex items-center gap-2 text-xs text-muted">
        <span className="relative flex h-1.5 w-1.5">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-teal opacity-60" />
          <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-teal" />
        </span>
        live
      </div>
    </aside>
  );
}
