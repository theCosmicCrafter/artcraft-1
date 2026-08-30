import { useEffect, useState } from "react";
import { twMerge } from "tailwind-merge";
import {
  CompanionInfo,
  GetCompanionHealth,
  GetCompanionStatus,
  RestartCompanion,
} from "@storyteller/tauri-api";
import { useCompanionStatusChangedEvent } from "@storyteller/tauri-events";

export interface CompanionStatusIndicatorProps {
  className?: string;
}

/** Result of independently verifying the Companion over HTTP, not just
 * trusting the supervisor's own "ready" status. */
type VerifyState = "unchecked" | "verified" | "unreachable";

/**
 * Small chrome indicator showing whether the local Companion Service is up.
 * The first real UI consumer of the Tauri<->Companion bridge
 * (GetCompanionStatus/RestartCompanion/useCompanionStatusChangedEvent) and
 * of @storyteller/design-tokens' `st-*` tokens - both built this same
 * session, this is what proves they compose correctly in an actual
 * component rather than just type-checking in isolation.
 *
 * Also the first UI consumer of `GetCompanionHealth` - once the supervisor
 * reports "ready" this independently calls the Companion's own health
 * endpoint over authenticated HTTP. That distinction matters: "ready" only
 * means the Rust supervisor's own health check passed, which doesn't prove
 * the *frontend* can reach the Companion (CSP, capability scope, and the
 * auth header are all separate failure points a Rust-side check can't see).
 *
 * Not the medialibrary grid/search UI itself (see UI_UX_PLAN.md and the
 * tracked backlog item for that) - this is deliberately the smallest useful
 * slice: something a host can mount today to show real Companion state.
 */
export const CompanionStatusIndicator = ({
  className,
}: CompanionStatusIndicatorProps) => {
  const [info, setInfo] = useState<CompanionInfo | null>(null);
  const [isRestarting, setIsRestarting] = useState(false);
  const [verifyState, setVerifyState] = useState<VerifyState>("unchecked");

  useEffect(() => {
    let isUnmounted = false;
    GetCompanionStatus().then((result) => {
      if (!isUnmounted) {
        setInfo(result);
      }
    });
    return () => {
      isUnmounted = true;
    };
  }, []);

  useCompanionStatusChangedEvent((updated) => {
    setInfo(updated);
    setIsRestarting(false);
  });

  useEffect(() => {
    if (info?.status.state !== "ready" || !info.port) {
      setVerifyState("unchecked");
      return;
    }

    let isUnmounted = false;
    setVerifyState("unchecked");
    GetCompanionHealth(info)
      .then(() => {
        if (!isUnmounted) setVerifyState("verified");
      })
      .catch(() => {
        if (!isUnmounted) setVerifyState("unreachable");
      });
    return () => {
      isUnmounted = true;
    };
    // Re-verify whenever the port changes (e.g. after a restart), not just
    // on every status_changed event for the same port.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [info?.status.state, info?.port]);

  const handleRestart = async () => {
    setIsRestarting(true);
    await RestartCompanion();
  };

  const { label, dotClassName, showRestart } = describeStatus(info, isRestarting, verifyState);

  return (
    <div
      className={twMerge(
        "flex items-center gap-2 rounded-st-control border border-st-hairline bg-st-canvas px-3 py-1.5 text-sm transition-colors",
        className
      )}
      style={{
        // Tailwind v3's shadow-[...] arbitrary-value parser misreads a bare
        // var() as a shadow *color* rather than a full shadow value (it has
        // no way to know what the variable resolves to), so `--st-inset-hi`
        // silently produced no box-shadow at all under this app's Tailwind
        // v3 config. Setting it directly here sidesteps that ambiguity -
        // same reason transition timing below is inline rather than a class.
        boxShadow: "var(--st-inset-hi)",
        transitionDuration: "var(--st-dur-fast)",
        transitionTimingFunction: "var(--st-ease)",
      }}
    >
      <span
        className={twMerge("h-2 w-2 rounded-full", dotClassName)}
        aria-hidden="true"
      />
      <span>{label}</span>
      {showRestart && (
        <button
          type="button"
          onClick={handleRestart}
          disabled={isRestarting}
          className="ml-1 rounded-st-control bg-st-accent px-2 py-0.5 text-xs text-white disabled:opacity-50"
        >
          {isRestarting ? "Restarting…" : "Restart"}
        </button>
      )}
    </div>
  );
};

function describeStatus(
  info: CompanionInfo | null,
  isRestarting: boolean,
  verifyState: VerifyState
): { label: string; dotClassName: string; showRestart: boolean } {
  if (isRestarting) {
    return { label: "Restarting…", dotClassName: "bg-yellow-500 animate-pulse", showRestart: false };
  }
  if (!info) {
    return { label: "Companion starting…", dotClassName: "bg-yellow-500 animate-pulse", showRestart: false };
  }
  switch (info.status.state) {
    case "starting":
      return { label: "Companion starting…", dotClassName: "bg-yellow-500 animate-pulse", showRestart: false };
    case "ready":
      // The supervisor's own health check passed, but that doesn't prove
      // *this* process (the webview) can reach it - CSP, capability scope,
      // and the auth header are all separate failure points. Distinguish
      // rather than silently claim "ready" for something unverified.
      switch (verifyState) {
        case "verified":
          return { label: "Companion ready", dotClassName: "bg-green-500", showRestart: false };
        case "unreachable":
          return { label: "Companion ready (unreachable from app)", dotClassName: "bg-orange-500", showRestart: true };
        case "unchecked":
        default:
          return { label: "Companion ready — verifying…", dotClassName: "bg-yellow-500 animate-pulse", showRestart: false };
      }
    case "exited":
      return info.status.restarting
        ? { label: "Companion restarting…", dotClassName: "bg-yellow-500 animate-pulse", showRestart: false }
        : { label: "Companion stopped", dotClassName: "bg-red-500", showRestart: true };
    case "failed_to_start":
      return { label: `Companion failed: ${info.status.reason}`, dotClassName: "bg-red-500", showRestart: true };
    default:
      return { label: "Companion status unknown", dotClassName: "bg-gray-400", showRestart: false };
  }
}
