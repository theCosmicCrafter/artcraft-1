import { companionFetch } from "./CompanionFetch";
import { CompanionInfo } from "./GetCompanionStatus";

// Partial mirror of companion/backend/models/api_models.py's MediaItemResponse
// - only the fields the medialibrary grid actually renders. The Companion's
// response has many more fields (markers, audio metadata, lineage, etc.);
// extras are simply ignored here, not an error.
export interface MediaItem {
  id: number;
  file_hash: string;
  file_format: string;
  width: number;
  height: number;
  vlm_caption: string | null;
  extracted_prompt: string | null;
  created_date: string | null;
}

// Mirrors MediaListResponse.
export interface MediaListResponse {
  items: MediaItem[];
  total: number;
  page: number;
  page_size: number;
}

export interface ListMediaParams {
  page?: number;
  pageSize?: number;
  captionQuery?: string;
  sortBy?: "created_desc" | "created_asc" | "indexed_desc" | "indexed_asc";
}

/**
 * Calls the Companion's `GET /api/media` - the real library-listing/search
 * endpoint (routes/media.py's `get_media`), not just the root health check
 * `GetCompanionHealth` already exercises. Only exposes the handful of query
 * params `MediaLibraryGrid` needs today (pagination + caption search); the
 * real endpoint supports dozens more (tags, folders, similarity search,
 * resolution/date filters, etc. - see `get_media`'s own signature) that a
 * fuller library UI would wire up incrementally, not all at once.
 */
export const ListMedia = async (
  info: CompanionInfo | null,
  params: ListMediaParams = {},
): Promise<MediaListResponse> => {
  const query = new URLSearchParams();
  query.set("page", String(params.page ?? 1));
  query.set("page_size", String(params.pageSize ?? 50));
  query.set("sort_by", params.sortBy ?? "created_desc");
  if (params.captionQuery) {
    query.set("caption_query", params.captionQuery);
  }

  const response = await companionFetch(info, `/api/media?${query.toString()}`);
  if (!response.ok) {
    throw new Error(`Listing media returned HTTP ${response.status}`);
  }
  return (await response.json()) as MediaListResponse;
};
