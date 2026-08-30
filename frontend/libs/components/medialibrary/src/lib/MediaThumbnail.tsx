import { useEffect, useState } from "react";
import { CompanionInfo, GetMediaThumbnailBlobUrl, MediaItem } from "@storyteller/tauri-api";

export interface MediaThumbnailProps {
  info: CompanionInfo | null;
  item: MediaItem;
  onClick?: (item: MediaItem) => void;
}

/**
 * One grid cell. Owns its own blob-URL lifecycle (fetch on mount, revoke on
 * unmount/item change) rather than the parent grid pre-fetching everything
 * up front - keeps memory bounded to what's actually rendered and matches
 * how a real virtualized/paginated grid would behave once one exists.
 */
export const MediaThumbnail = ({ info, item, onClick }: MediaThumbnailProps) => {
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let isUnmounted = false;
    let url: string | null = null;
    setFailed(false);

    GetMediaThumbnailBlobUrl(info, item.id)
      .then((result) => {
        if (isUnmounted) {
          URL.revokeObjectURL(result);
          return;
        }
        url = result;
        setBlobUrl(result);
      })
      .catch(() => {
        if (!isUnmounted) setFailed(true);
      });

    return () => {
      isUnmounted = true;
      if (url) URL.revokeObjectURL(url);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [info?.port, item.id]);

  const caption = item.vlm_caption ?? item.extracted_prompt ?? undefined;

  return (
    <button
      type="button"
      onClick={() => onClick?.(item)}
      title={caption}
      className="group relative aspect-square overflow-hidden rounded-st-card border border-st-hairline bg-st-canvas transition-colors"
    >
      {blobUrl ? (
        <img
          src={blobUrl}
          alt={caption ?? `Media ${item.id}`}
          className="h-full w-full object-cover"
          loading="lazy"
        />
      ) : failed ? (
        <div className="flex h-full w-full items-center justify-center text-xs text-red-400">
          Failed to load
        </div>
      ) : (
        <div className="h-full w-full animate-pulse bg-st-hairline" />
      )}
    </button>
  );
};
