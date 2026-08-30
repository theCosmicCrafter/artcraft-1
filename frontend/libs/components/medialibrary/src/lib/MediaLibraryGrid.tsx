import { useEffect, useState } from "react";
import { twMerge } from "tailwind-merge";
import {
  CompanionInfo,
  GetCompanionStatus,
  ListMedia,
  MediaItem,
} from "@storyteller/tauri-api";
import { useCompanionStatusChangedEvent } from "@storyteller/tauri-events";
import { MediaThumbnail } from "./MediaThumbnail";

export interface MediaLibraryGridProps {
  className?: string;
  onSelect?: (item: MediaItem) => void;
  refreshToken?: number;
}

const PAGE_SIZE = 50;
const SEARCH_DEBOUNCE_MS = 400;

/**
 * The first real medialibrary screen - a searchable, paginated grid backed
 * by the Companion's actual `GET /api/media` (via `ListMedia`), not a mock.
 * Everything before this (`CompanionStatusIndicator`) only proved the
 * Companion was reachable; this is the first component that does something
 * a user would call "the library."
 *
 * Deliberately not virtualized/infinite-scroll yet (plain prev/next
 * pagination) and only wires caption search + one sort order out of the
 * dozens of filters `get_media` supports server-side - see UI_UX_PLAN.md §5
 * for the still-open information-architecture questions (where this mounts
 * relative to generation pages, face-cluster UI, mature-content display
 * treatment) that a fuller pass should resolve. This is the smallest slice
 * that's genuinely useful: type to search, click to select.
 */
export const MediaLibraryGrid = ({
  className,
  onSelect,
  refreshToken = 0,
}: MediaLibraryGridProps) => {
  const [info, setInfo] = useState<CompanionInfo | null>(null);
  const [searchInput, setSearchInput] = useState("");
  const [debouncedQuery, setDebouncedQuery] = useState("");
  const [page, setPage] = useState(1);
  const [result, setResult] = useState<{ items: MediaItem[]; total: number } | null>(null);
  const [loadState, setLoadState] = useState<"idle" | "loading" | "error">("idle");

  useEffect(() => {
    let isUnmounted = false;
    GetCompanionStatus()
      .then((status) => {
        if (!isUnmounted) setInfo(status);
      })
      .catch(() => {
        if (!isUnmounted) setInfo(null);
      });
    return () => {
      isUnmounted = true;
    };
  }, []);

  useCompanionStatusChangedEvent((updated) => setInfo(updated));

  // Debounce free-text search so every keystroke doesn't fire a request.
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedQuery(searchInput), SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [searchInput]);

  // Reset to page 1 whenever the search term changes - staying on e.g. page
  // 4 of a new, narrower result set would just show "no results" confusingly.
  useEffect(() => {
    setPage(1);
  }, [debouncedQuery]);

  useEffect(() => {
    if (info?.status.state !== "ready" || !info.port) {
      return;
    }

    let isUnmounted = false;
    setLoadState("loading");
    ListMedia(info, { page, pageSize: PAGE_SIZE, captionQuery: debouncedQuery || undefined })
      .then((response) => {
        if (isUnmounted) return;
        setResult({ items: response.items, total: response.total });
        setLoadState("idle");
      })
      .catch(() => {
        if (!isUnmounted) setLoadState("error");
      });
    return () => {
      isUnmounted = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [info?.status.state, info?.port, page, debouncedQuery, refreshToken]);

  const totalPages = result ? Math.max(1, Math.ceil(result.total / PAGE_SIZE)) : 1;

  return (
    <div className={twMerge("flex h-full flex-col gap-4", className)}>
      <input
        type="search"
        value={searchInput}
        onChange={(e) => setSearchInput(e.target.value)}
        placeholder="Search captions…"
        className="rounded-st-control border border-st-hairline bg-st-canvas px-3 py-2 text-sm outline-none"
        disabled={info?.status.state !== "ready"}
      />

      <MediaLibraryGridBody
        info={info}
        loadState={loadState}
        result={result}
        onSelect={onSelect}
      />

      {result && result.total > 0 && (
        <div className="flex items-center justify-center gap-3 text-sm">
          <button
            type="button"
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={page <= 1}
            className="rounded-st-control border border-st-hairline px-3 py-1 disabled:opacity-40"
          >
            Previous
          </button>
          <span>
            Page {page} of {totalPages} ({result.total} items)
          </span>
          <button
            type="button"
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            disabled={page >= totalPages}
            className="rounded-st-control border border-st-hairline px-3 py-1 disabled:opacity-40"
          >
            Next
          </button>
        </div>
      )}
    </div>
  );
};

interface MediaLibraryGridBodyProps {
  info: CompanionInfo | null;
  loadState: "idle" | "loading" | "error";
  result: { items: MediaItem[]; total: number } | null;
  onSelect?: (item: MediaItem) => void;
}

function MediaLibraryGridBody({ info, loadState, result, onSelect }: MediaLibraryGridBodyProps) {
  if (info?.status.state === "starting" || !info) {
    return <CenteredMessage>Waiting for the Companion to start…</CenteredMessage>;
  }
  if (info.status.state === "failed_to_start") {
    return <CenteredMessage>Companion failed to start: {info.status.reason}</CenteredMessage>;
  }
  if (info.status.state === "exited") {
    return (
      <CenteredMessage>
        {info.status.restarting ? "Companion restarting…" : "Companion stopped."}
      </CenteredMessage>
    );
  }
  if (loadState === "loading" && !result) {
    return <CenteredMessage>Loading…</CenteredMessage>;
  }
  if (loadState === "error") {
    return <CenteredMessage>Couldn't reach the Companion's library. Try again shortly.</CenteredMessage>;
  }
  if (result && result.items.length === 0) {
    return <CenteredMessage>No media found.</CenteredMessage>;
  }

  return (
    <div className="grid grid-cols-2 gap-3 overflow-y-auto sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6">
      {result?.items.map((item) => (
        <MediaThumbnail key={item.id} info={info} item={item} onClick={onSelect} />
      ))}
    </div>
  );
}

function CenteredMessage({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-1 items-center justify-center text-sm text-base-fg/60">
      {children}
    </div>
  );
}
