import { companionFetch } from "./CompanionFetch";
import { CompanionInfo } from "./GetCompanionStatus";

/**
 * Fetches a media item's thumbnail and returns a `blob:` object URL for it.
 *
 * Can't just point an `<img src="http://127.0.0.1:{port}/...">` at the
 * Companion directly - `CompanionAuthMiddleware` requires the shared-secret
 * header on every request (including thumbnails), and an `<img>` tag has no
 * way to attach custom headers to the request it issues. Routing the bytes
 * through authenticated `companionFetch` first, then handing the browser a
 * local blob URL, is the only way to load an authenticated image via a
 * normal `<img>`/CSS `background-image`.
 *
 * Caller owns the returned URL's lifetime - call `URL.revokeObjectURL` on it
 * when done (e.g. on unmount/re-fetch) or it leaks for the page's lifetime.
 */
export const GetMediaThumbnailBlobUrl = async (
  info: CompanionInfo | null,
  mediaId: number,
  size = 256,
): Promise<string> => {
  const response = await companionFetch(info, `/api/media/${mediaId}/thumbnail?size=${size}`);
  if (!response.ok) {
    throw new Error(`Fetching thumbnail for media ${mediaId} returned HTTP ${response.status}`);
  }
  const blob = await response.blob();
  return URL.createObjectURL(blob);
};
