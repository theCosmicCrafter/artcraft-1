import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@storyteller/ui-button";
import { Modal } from "@storyteller/ui-modal";
import {
  CompanionInfo,
  GetCompanionStatus,
  ImportEagleVault,
  ImportEagleVaultResult,
  ScanEagleVault,
  ScanEagleVaultResult,
} from "@storyteller/tauri-api";
import { useCompanionStatusChangedEvent } from "@storyteller/tauri-events";

type WizardStep = "pick" | "scanning" | "confirm" | "importing" | "complete" | "error";

export interface EagleImportWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onImported?: (result: ImportEagleVaultResult) => void;
}

export const EagleImportWizard = ({
  isOpen,
  onClose,
  onImported,
}: EagleImportWizardProps) => {
  const [step, setStep] = useState<WizardStep>("pick");
  const [companionInfo, setCompanionInfo] = useState<CompanionInfo | null>(null);
  const [vaultPath, setVaultPath] = useState("");
  const [scanResult, setScanResult] = useState<ScanEagleVaultResult | null>(null);
  const [importResult, setImportResult] = useState<ImportEagleVaultResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useCompanionStatusChangedEvent((updated) => setCompanionInfo(updated));

  useEffect(() => {
    if (!isOpen) return;

    let isUnmounted = false;
    setStep("pick");
    setVaultPath("");
    setScanResult(null);
    setImportResult(null);
    setError(null);

    GetCompanionStatus()
      .then((status) => {
        if (!isUnmounted) setCompanionInfo(status);
      })
      .catch(() => {
        if (!isUnmounted) setCompanionInfo(null);
      });

    return () => {
      isUnmounted = true;
    };
  }, [isOpen]);

  const closeWizard = () => {
    if (step === "importing") return;
    setStep("pick");
    setVaultPath("");
    setScanResult(null);
    setImportResult(null);
    setError(null);
    onClose();
  };

  const scanVault = async (path: string) => {
    setStep("scanning");
    setError(null);

    try {
      const info = await getReadyCompanion(companionInfo);
      const result = await ScanEagleVault(info, path);
      setCompanionInfo(info);
      setScanResult(result);
      setStep("confirm");
    } catch (caughtError) {
      setError(toErrorMessage(caughtError, "Could not scan this Eagle library."));
      setStep("error");
    }
  };

  const pickVault = async () => {
    setError(null);

    try {
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected === "string") {
        setVaultPath(selected);
        await scanVault(selected);
      }
    } catch (caughtError) {
      setError(toErrorMessage(caughtError, "Could not open the directory picker."));
      setStep("error");
    }
  };

  const importVault = async () => {
    if (!vaultPath) return;

    setStep("importing");
    setError(null);

    try {
      const info = await getReadyCompanion(companionInfo);
      const result = await ImportEagleVault(info, vaultPath);
      setCompanionInfo(info);
      setImportResult(result);
      setStep("complete");
      onImported?.(result);
    } catch (caughtError) {
      setError(toErrorMessage(caughtError, "Could not import this Eagle library."));
      setStep("error");
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={closeWizard}
      title="Import Eagle Library"
      width={640}
      closeOnOutsideClick={step !== "importing"}
      closeOnEsc={step !== "importing"}
    >
      <div className="flex flex-col gap-5 p-5 text-base-fg">
        {step === "pick" && <PickStep onPick={pickVault} />}
        {step === "scanning" && <WorkingStep label="Scanning Eagle library…" />}
        {step === "confirm" && scanResult && (
          <ConfirmStep
            path={vaultPath}
            result={scanResult}
            onBack={() => setStep("pick")}
            onImport={importVault}
          />
        )}
        {step === "importing" && <WorkingStep label="Importing assets into the local library…" />}
        {step === "complete" && importResult && (
          <CompleteStep result={importResult} onClose={closeWizard} />
        )}
        {step === "error" && (
          <ErrorStep
            message={error ?? "The import could not be completed."}
            onRetry={() => {
              setError(null);
              setStep("pick");
            }}
            onClose={closeWizard}
          />
        )}
      </div>
    </Modal>
  );
};

const PickStep = ({ onPick }: { onPick: () => void }) => (
  <div className="flex flex-col gap-4">
    <div>
      <h2 className="text-lg font-semibold">Bring your Eagle library into Studio</h2>
      <p className="mt-2 text-sm text-base-fg/70">
        Studio copies your media into managed local storage, preserves Eagle tags and folders,
        and never modifies the original .library folder.
      </p>
    </div>
    <div className="rounded-st-card border border-st-hairline bg-st-canvas p-4 text-sm text-base-fg/70">
      Semantic embeddings are recomputed locally after import. Eagle&apos;s own vector index is not
      copied because it uses a different embedding space.
    </div>
    <div className="flex justify-end">
      <Button onClick={onPick}>Choose .library folder</Button>
    </div>
  </div>
);

const ConfirmStep = ({
  path,
  result,
  onBack,
  onImport,
}: {
  path: string;
  result: ScanEagleVaultResult;
  onBack: () => void;
  onImport: () => void;
}) => (
  <div className="flex flex-col gap-5">
    <div>
      <h2 className="text-lg font-semibold">Review import</h2>
      <p className="mt-2 break-all text-xs text-base-fg/60">{path}</p>
    </div>
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
      <SummaryCard label="Items" value={result.item_count} />
      <SummaryCard label="Folders" value={result.folder_count} />
      <SummaryCard label="Tags" value={result.tag_count} />
      <SummaryCard label="Deleted" value={result.deleted_item_count} />
    </div>
    {result.top_level_folders.length > 0 && (
      <div>
        <p className="mb-2 text-sm font-medium">Top-level folders</p>
        <div className="flex max-h-28 flex-wrap gap-2 overflow-y-auto">
          {result.top_level_folders.map((folder) => (
            <span
              key={folder.id}
              className="rounded-st-control border border-st-hairline bg-st-canvas px-2 py-1 text-xs"
            >
              {folder.name} <span className="text-base-fg/50">({folder.child_count})</span>
            </span>
          ))}
        </div>
      </div>
    )}
    <div className="flex justify-end gap-2">
      <Button variant="secondary" onClick={onBack}>
        Choose another
      </Button>
      <Button onClick={onImport}>Import {result.item_count.toLocaleString()} items</Button>
    </div>
  </div>
);

const WorkingStep = ({ label }: { label: string }) => (
  <div className="flex flex-col gap-4 py-8">
    <p className="text-center text-sm">{label}</p>
    <div
      className="h-2 w-full overflow-hidden rounded-full bg-st-canvas"
      role="progressbar"
      aria-label={label}
      aria-valuetext="Working"
    >
      <div className="h-full w-1/3 animate-pulse rounded-full bg-st-accent" />
    </div>
    <p className="text-center text-xs text-base-fg/60">
      This can take time for large libraries. Keep the original Eagle folder available until the
      import completes.
    </p>
  </div>
);

const CompleteStep = ({
  result,
  onClose,
}: {
  result: ImportEagleVaultResult;
  onClose: () => void;
}) => (
  <div className="flex flex-col gap-5">
    <div>
      <h2 className="text-lg font-semibold">Import complete</h2>
      <p className="mt-2 text-sm text-base-fg/70">
        The imported files are now in Studio&apos;s local library. Background indexing will continue
        for semantic search and captions.
      </p>
    </div>
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
      <SummaryCard label="Imported" value={result.imported} />
      <SummaryCard label="Already imported" value={result.already_imported} />
      <SummaryCard label="Deleted skipped" value={result.skipped_deleted} />
      <SummaryCard label="Missing skipped" value={result.skipped_missing_file} />
      <SummaryCard label="Tags touched" value={result.tags_touched} />
      <SummaryCard label="Boards touched" value={result.boards_touched} />
    </div>
    {result.skipped_error > 0 && (
      <p className="text-sm text-orange-300">
        {result.skipped_error.toLocaleString()} items were skipped because of import errors.
      </p>
    )}
    <div className="flex justify-end">
      <Button onClick={onClose}>Done</Button>
    </div>
  </div>
);

const ErrorStep = ({
  message,
  onRetry,
  onClose,
}: {
  message: string;
  onRetry: () => void;
  onClose: () => void;
}) => (
  <div className="flex flex-col gap-5">
    <div>
      <h2 className="text-lg font-semibold">Import could not continue</h2>
      <p className="mt-2 text-sm text-red-300">{message}</p>
    </div>
    <div className="flex justify-end gap-2">
      <Button variant="secondary" onClick={onClose}>
        Close
      </Button>
      <Button onClick={onRetry}>Try again</Button>
    </div>
  </div>
);

const SummaryCard = ({ label, value }: { label: string; value: number }) => (
  <div className="rounded-st-card border border-st-hairline bg-st-canvas p-3">
    <p className="text-xs text-base-fg/55">{label}</p>
    <p className="mt-1 text-lg font-semibold tabular-nums">{value.toLocaleString()}</p>
  </div>
);

const getReadyCompanion = async (current: CompanionInfo | null): Promise<CompanionInfo> => {
  const info = current ?? (await GetCompanionStatus());
  if (!info || info.status.state !== "ready" || !info.port || !info.auth_token) {
    throw new Error("The Companion is not ready. Start or restart it, then try again.");
  }
  return info;
};

const toErrorMessage = (error: unknown, fallback: string): string => {
  if (error instanceof Error && error.message) return error.message;
  return fallback;
};
