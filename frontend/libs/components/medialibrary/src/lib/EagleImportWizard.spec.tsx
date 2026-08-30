import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EagleImportWizard } from "./EagleImportWizard";

const mocks = vi.hoisted(() => ({
  getStatus: vi.fn(),
  importVault: vi.fn(),
  open: vi.fn(),
  scanVault: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open }));
vi.mock("@storyteller/tauri-events", () => ({
  useCompanionStatusChangedEvent: vi.fn(),
}));
vi.mock("@storyteller/tauri-api", () => ({
  GetCompanionStatus: mocks.getStatus,
  ImportEagleVault: mocks.importVault,
  ScanEagleVault: mocks.scanVault,
}));
vi.mock("@storyteller/ui-modal", () => ({
  Modal: ({
    children,
    isOpen,
    title,
  }: {
    children: React.ReactNode;
    isOpen: boolean;
    title: string;
  }) => (isOpen ? <div role="dialog" aria-label={title}>{children}</div> : null),
}));
vi.mock("@storyteller/ui-button", () => ({
  Button: ({
    children,
    ...props
  }: React.ButtonHTMLAttributes<HTMLButtonElement>) => <button {...props}>{children}</button>,
}));

describe("EagleImportWizard", () => {
  it("scans, confirms, imports, and reports the result", async () => {
    const companion = {
      port: 43123,
      auth_token: "test-token",
      status: { state: "ready" },
    };
    const scanResult = {
      vault_path: "C:\\Library.library",
      item_count: 12,
      folder_count: 3,
      deleted_item_count: 1,
      tag_count: 8,
      top_level_folders: [{ id: "folder-1", name: "References", child_count: 2 }],
    };
    const importResult = {
      imported: 11,
      already_imported: 0,
      skipped_deleted: 1,
      skipped_missing_file: 0,
      skipped_error: 0,
      tags_touched: 8,
      boards_touched: 3,
    };
    const onImported = vi.fn();

    mocks.getStatus.mockResolvedValue(companion);
    mocks.open.mockResolvedValue("C:\\Library.library");
    mocks.scanVault.mockResolvedValue(scanResult);
    mocks.importVault.mockResolvedValue(importResult);

    render(
      <EagleImportWizard
        isOpen
        onClose={vi.fn()}
        onImported={onImported}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Choose .library folder" }));

    await screen.findByText("Review import");
    expect(screen.getByText("References")).toBeTruthy();
    expect(mocks.scanVault).toHaveBeenCalledWith(companion, "C:\\Library.library");

    fireEvent.click(screen.getByRole("button", { name: "Import 12 items" }));

    await screen.findByText("Import complete");
    await waitFor(() => expect(onImported).toHaveBeenCalledWith(importResult));
    expect(mocks.importVault).toHaveBeenCalledWith(companion, "C:\\Library.library");
  });

  it("shows a useful error when the Companion is not ready", async () => {
    mocks.getStatus.mockResolvedValue(null);
    mocks.open.mockResolvedValue("C:\\Library.library");

    render(<EagleImportWizard isOpen onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole("button", { name: "Choose .library folder" }));

    expect(await screen.findByText("Import could not continue")).toBeTruthy();
    expect(screen.getByText(/Companion is not ready/)).toBeTruthy();
  });
});
