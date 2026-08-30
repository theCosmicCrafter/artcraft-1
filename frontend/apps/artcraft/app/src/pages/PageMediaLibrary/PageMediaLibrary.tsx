import { useState } from "react";
import { faFolderOpen } from "@fortawesome/pro-solid-svg-icons";
import { Button } from "@storyteller/ui-button";
import {
  EagleImportWizard,
  MediaLibraryGrid,
} from "@storyteller/ui-medialibrary";

// Desktop media library tab - the first real medialibrary screen (search +
// paginated grid backed by the Companion's actual library index), reachable
// as its own top-level app like Create Image/Create Video. See
// UI_UX_PLAN.md §5 for the still-open questions a fuller design pass should
// resolve (exact IA relative to generation pages, face-cluster UI, mature-
// content display treatment) - this is a deliberately minimal first cut,
// not that pass.
export const PageMediaLibrary = () => {
  const [isImportWizardOpen, setIsImportWizardOpen] = useState(false);
  const [refreshToken, setRefreshToken] = useState(0);

  return (
    <div className="fixed inset-0 bg-ui-background pt-[56px] text-base-fg">
      <div className="flex items-center justify-between border-b border-ui-panel-border px-6 py-3">
        <div>
          <h1 className="text-lg font-semibold">Media Library</h1>
          <p className="text-xs text-base-fg/55">
            Search, organize, and import your local creative archive.
          </p>
        </div>
        <Button
          icon={faFolderOpen}
          variant="secondary"
          onClick={() => setIsImportWizardOpen(true)}
        >
          Import Eagle library
        </Button>
      </div>
      <MediaLibraryGrid
        refreshToken={refreshToken}
        className="h-[calc(100%-78px)] px-6 py-4"
      />
      <EagleImportWizard
        isOpen={isImportWizardOpen}
        onClose={() => setIsImportWizardOpen(false)}
        onImported={() => setRefreshToken((value) => value + 1)}
      />
    </div>
  );
};
